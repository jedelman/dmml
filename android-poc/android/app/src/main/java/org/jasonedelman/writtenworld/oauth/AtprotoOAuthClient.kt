package org.jasonedelman.writtenworld.oauth

import android.content.Context
import android.net.Uri
import androidx.browser.customtabs.CustomTabsIntent
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.Base64

// Real atproto OAuth (PAR + PKCE + DPoP), per https://atproto.com/specs/oauth
// and https://docs.bsky.app/docs/advanced-guides/oauth-client, verified
// against those documents before writing any of this (not assumed from
// generic OAuth knowledge -- atproto has real, specific requirements:
// PAR is mandatory, not optional; DPoP is mandatory for every client
// type; redirect_uri for a native client must be a custom scheme in
// reverse-domain order matching client_id's own host).
//
// This is a PUBLIC client (token_endpoint_auth_method: none in
// client-metadata.json -- see jason-edelman.org/oauth/written-world-android/
// client-metadata.json) -- no client secret exists anywhere in this
// app, by design; DPoP possession-proof is the whole security model
// for a client shaped like this.
object AtprotoOAuthClient {
    const val CLIENT_ID = "https://jason-edelman.org/oauth/written-world-android/client-metadata.json"
    // Real, server-confirmed constraint (not a convention we chose):
    // atproto requires a private-use-scheme redirect_uri to be
    // EXACTLY the client_id's FQDN in reverse order, no extra path
    // segments -- a first attempt with an extra ".written-world"
    // segment was rejected by the real authorization server with
    // "Private-Use URI Scheme redirect URI ... must be the fully
    // qualified domain name (FQDN) of the client_id, in reverse order
    // (org.jason-edelman:)".
    const val REDIRECT_URI = "org.jason-edelman:/callback"
    private const val SCOPE = "atproto transition:generic"

    private val http = OkHttpClient()
    private val JSON_MEDIA_TYPE = "application/json".toMediaType()
    private val FORM_MEDIA_TYPE = "application/x-www-form-urlencoded".toMediaType()

    data class PendingAuth(
        val state: String,
        val codeVerifier: String,
        val tokenEndpoint: String,
        val authServerIssuer: String,
        val pdsEndpoint: String,
        val did: String,
    )

    private fun b64url(bytes: ByteArray): String =
        Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)

    private fun randomUrlSafeString(byteLength: Int): String {
        val bytes = ByteArray(byteLength)
        SecureRandom().nextBytes(bytes)
        return b64url(bytes)
    }

    /**
     * Step 1: resolve `identifier` (handle or DID) to a real DID + PDS
     * endpoint via the EXISTING NativeBridge.atprotoResolve upcall --
     * reused rather than re-implemented, since real DID/handle
     * resolution already works and is verified (see
     * dev-journal/2026-09-08-android-jni-upcall-verified-on-device.md).
     */
    private fun resolveIdentifier(identifier: String): Pair<String, String> {
        val result = org.jasonedelman.writtenworld.NativeBridge.atprotoResolve(identifier)
        if (org.jasonedelman.writtenworld.NativeBridge.isError(result)) {
            throw IllegalStateException("atprotoResolve failed: $result")
        }
        val json = JSONObject(result)
        return json.getString("did") to json.getString("pdsEndpoint")
    }

    /** Step 2-3: PDS -> protected-resource metadata -> authorization
     * server issuer -> authorization server metadata. Two real HTTP
     * hops, per spec -- the PDS itself is a resource server, not the
     * authorization server; it only points at the AS, which is
     * commonly (not always) the same operator's separate endpoint. */
    private fun discoverAuthServerMetadata(pdsEndpoint: String): JSONObject {
        val protectedResourceReq = Request.Builder()
            .url("$pdsEndpoint/.well-known/oauth-protected-resource")
            .build()
        val protectedResource = http.newCall(protectedResourceReq).execute().use { resp ->
            if (!resp.isSuccessful) throw IllegalStateException("oauth-protected-resource fetch failed: ${resp.code}")
            JSONObject(resp.body!!.string())
        }
        val authServerIssuer = protectedResource.getJSONArray("authorization_servers").getString(0)

        val asMetadataReq = Request.Builder()
            .url("$authServerIssuer/.well-known/oauth-authorization-server")
            .build()
        return http.newCall(asMetadataReq).execute().use { resp ->
            if (!resp.isSuccessful) throw IllegalStateException("oauth-authorization-server fetch failed: ${resp.code}")
            JSONObject(resp.body!!.string()).put("__issuer", authServerIssuer)
        }
    }

    /** Step 4-5: PKCE + a Pushed Authorization Request, with the real
     * DPoP-nonce retry atproto's authorization servers commonly
     * require in practice (first attempt returns 400 use_dpop_nonce +
     * a DPoP-Nonce response header; retry once with that nonce in the
     * proof). Returns the PAR `request_uri` to embed in the
     * authorization URL. */
    private fun pushAuthorizationRequest(
        parEndpoint: String,
        state: String,
        codeChallenge: String,
        loginHint: String,
    ): String {
        val form = "response_type=code" +
            "&client_id=${Uri.encode(CLIENT_ID)}" +
            "&redirect_uri=${Uri.encode(REDIRECT_URI)}" +
            "&scope=${Uri.encode(SCOPE)}" +
            "&state=${Uri.encode(state)}" +
            "&code_challenge=${Uri.encode(codeChallenge)}" +
            "&code_challenge_method=S256" +
            "&login_hint=${Uri.encode(loginHint)}"

        fun attempt(nonce: String?): okhttp3.Response {
            val proof = DpopKeyManager.createProof("POST", parEndpoint, nonce = nonce)
            val req = Request.Builder()
                .url(parEndpoint)
                .header("DPoP", proof)
                .post(form.toRequestBody(FORM_MEDIA_TYPE))
                .build()
            return http.newCall(req).execute()
        }

        var resp = attempt(null)
        if (!resp.isSuccessful) {
            val bodyStr = resp.body?.string().orEmpty()
            val retryNonce = resp.header("DPoP-Nonce")
            if (resp.code == 400 && retryNonce != null && bodyStr.contains("use_dpop_nonce")) {
                resp.close()
                resp = attempt(retryNonce)
            } else {
                resp.close()
                throw IllegalStateException("PAR failed: ${resp.code} $bodyStr")
            }
        }
        return resp.use { r ->
            if (!r.isSuccessful) throw IllegalStateException("PAR failed after nonce retry: ${r.code} ${r.body?.string()}")
            JSONObject(r.body!!.string()).getString("request_uri")
        }
    }

    /**
     * Full flow through PAR: resolves the identifier, discovers AS
     * metadata, generates a PKCE pair, pushes the authorization
     * request, and returns both the URL to open in a Custom Tab and
     * the state this flow needs to complete the redirect later.
     * `context` launches the Custom Tab; the actual redirect is caught
     * by OAuthCallbackActivity, which must be handed `pending` (e.g.
     * via a short-lived in-memory holder -- see OAuthLoginScreen) to
     * finish the exchange.
     */
    fun beginLogin(context: Context, identifier: String): PendingAuth {
        DpopKeyManager.ensureKeyExists()
        val (did, pdsEndpoint) = resolveIdentifier(identifier)
        val asMetadata = discoverAuthServerMetadata(pdsEndpoint)
        val authServerIssuer = asMetadata.getString("__issuer")
        val parEndpoint = asMetadata.getString("pushed_authorization_request_endpoint")
        val authorizationEndpoint = asMetadata.getString("authorization_endpoint")
        val tokenEndpoint = asMetadata.getString("token_endpoint")

        val state = randomUrlSafeString(24)
        val codeVerifier = randomUrlSafeString(48)
        val codeChallenge = b64url(MessageDigest.getInstance("SHA-256").digest(codeVerifier.toByteArray(Charsets.UTF_8)))

        val requestUri = pushAuthorizationRequest(parEndpoint, state, codeChallenge, identifier)

        val authUrl = "$authorizationEndpoint?client_id=${Uri.encode(CLIENT_ID)}&request_uri=${Uri.encode(requestUri)}"
        CustomTabsIntent.Builder().build().launchUrl(context, Uri.parse(authUrl))

        return PendingAuth(state, codeVerifier, tokenEndpoint, authServerIssuer, pdsEndpoint, did)
    }

    /**
     * Step 6-7: called by OAuthCallbackActivity once the redirect
     * lands with `code`/`state`. Verifies `state` matches, then
     * exchanges `code` for tokens -- same DPoP-nonce retry as PAR,
     * since the token endpoint independently enforces its own nonce.
     */
    fun completeLogin(pending: PendingAuth, redirectState: String, code: String): OAuthTokenStore.Session {
        require(redirectState == pending.state) { "OAuth state mismatch -- possible CSRF, refusing to exchange code" }

        val form = "grant_type=authorization_code" +
            "&code=${Uri.encode(code)}" +
            "&redirect_uri=${Uri.encode(REDIRECT_URI)}" +
            "&client_id=${Uri.encode(CLIENT_ID)}" +
            "&code_verifier=${Uri.encode(pending.codeVerifier)}"

        fun attempt(nonce: String?): okhttp3.Response {
            val proof = DpopKeyManager.createProof("POST", pending.tokenEndpoint, nonce = nonce)
            val req = Request.Builder()
                .url(pending.tokenEndpoint)
                .header("DPoP", proof)
                .post(form.toRequestBody(FORM_MEDIA_TYPE))
                .build()
            return http.newCall(req).execute()
        }

        var resp = attempt(null)
        if (!resp.isSuccessful) {
            val bodyStr = resp.body?.string().orEmpty()
            val retryNonce = resp.header("DPoP-Nonce")
            if (resp.code == 400 && retryNonce != null && bodyStr.contains("use_dpop_nonce")) {
                resp.close()
                resp = attempt(retryNonce)
            } else {
                resp.close()
                throw IllegalStateException("token exchange failed: ${resp.code} $bodyStr")
            }
        }
        val json = resp.use { r ->
            if (!r.isSuccessful) throw IllegalStateException("token exchange failed after nonce retry: ${r.code} ${r.body?.string()}")
            JSONObject(r.body!!.string())
        }

        return OAuthTokenStore.Session(
            accessToken = json.getString("access_token"),
            refreshToken = json.getString("refresh_token"),
            did = pending.did,
            pdsEndpoint = pending.pdsEndpoint,
            authServerIssuer = pending.authServerIssuer,
        )
    }
}
