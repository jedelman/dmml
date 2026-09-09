package org.jasonedelman.writtenworld.oauth

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import org.json.JSONObject
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.MessageDigest
import java.security.Signature
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec
import java.util.Base64
import java.util.UUID

// DPoP (RFC 9449) key management and proof-JWT construction for atproto
// OAuth. atproto's real requirement, confirmed against the spec before
// writing any of this: DPoP is mandatory for EVERY client type, for
// both the token endpoint AND every subsequent resource-server (PDS)
// request -- not an opt-in, not just login. This file only covers key
// management + proof construction; actually attaching a DPoP proof to
// every DMML.Atproto call after login (via DMML.Http) is real,
// disclosed follow-up work, not done here -- see the login-screen dev
// journal entry.
//
// The private key NEVER leaves AndroidKeyStore -- generated there,
// used only via Signature.initSign(keystoreEntry), never exported as
// raw bytes. Only the PUBLIC key's coordinates are ever read out (to
// build the JWK embedded in each proof's header), which is the whole
// point of DPoP: the server can verify possession of a key it never
// sees the private half of.
object DpopKeyManager {
    private const val KEYSTORE_ALIAS = "atproto_dpop_key"
    private const val ANDROID_KEYSTORE = "AndroidKeyStore"

    private fun keyStore(): KeyStore =
        KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }

    /** Generates the DPoP keypair once, on first use; reuses it after
     * that (the whole point of an EC keypair here is a stable public
     * key the authorization server associates with this client
     * instance's tokens -- generating a fresh one per request would
     * break every already-issued DPoP-bound token). */
    fun ensureKeyExists() {
        val ks = keyStore()
        if (ks.containsAlias(KEYSTORE_ALIAS)) return
        val generator = KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, ANDROID_KEYSTORE)
        generator.initialize(
            KeyGenParameterSpec.Builder(KEYSTORE_ALIAS, KeyProperties.PURPOSE_SIGN)
                .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
                .setDigests(KeyProperties.DIGEST_SHA256)
                .build()
        )
        generator.generateKeyPair()
    }

    private fun publicKey(): ECPublicKey {
        ensureKeyExists()
        val ks = keyStore()
        return ks.getCertificate(KEYSTORE_ALIAS).publicKey as ECPublicKey
    }

    private fun privateKeyEntry(): KeyStore.PrivateKeyEntry {
        ensureKeyExists()
        val ks = keyStore()
        return ks.getEntry(KEYSTORE_ALIAS, null) as KeyStore.PrivateKeyEntry
    }

    private fun b64url(bytes: ByteArray): String =
        Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)

    // P-256 field elements are 32 bytes; BigInteger.toByteArray() can
    // add a leading zero sign byte or come up short for small values --
    // both real, not hypothetical, since JWK's x/y MUST be exactly
    // 32 bytes, unsigned, big-endian.
    private fun fixedLength32(unsigned: java.math.BigInteger): ByteArray {
        val raw = unsigned.toByteArray()
        val trimmed = if (raw.size > 32 && raw[0].toInt() == 0) raw.copyOfRange(1, raw.size) else raw
        return if (trimmed.size == 32) trimmed else ByteArray(32 - trimmed.size) + trimmed
    }

    /** The public key as a JWK (JSON Web Key), embedded in every DPoP
     * proof's header -- this, not a separate registration step, is how
     * the server learns this client's public key at all. */
    fun publicJwk(): JSONObject {
        val pub = publicKey()
        val point = pub.w
        return JSONObject()
            .put("kty", "EC")
            .put("crv", "P-256")
            .put("x", b64url(fixedLength32(point.affineX)))
            .put("y", b64url(fixedLength32(point.affineY)))
    }

    /** RFC 7638 JWK thumbprint -- required for the `jkt` confirmation
     * claim some flows check, and useful as a stable client-key id. */
    fun jwkThumbprint(): String {
        val jwk = publicJwk()
        // Canonical form for thumbprinting: exactly these three
        // members, in this lexicographic order, no whitespace.
        val canonical = "{\"crv\":\"${jwk.getString("crv")}\",\"kty\":\"${jwk.getString("kty")}\",\"x\":\"${jwk.getString("x")}\",\"y\":\"${jwk.getString("y")}\"}"
        val digest = MessageDigest.getInstance("SHA-256").digest(canonical.toByteArray(Charsets.UTF_8))
        return b64url(digest)
    }

    // Java's Signature.sign() for "SHA256withECDSA" returns a
    // DER-encoded ASN.1 SEQUENCE(INTEGER r, INTEGER s) -- a real,
    // well-known JWS gotcha, not this implementation's own invention:
    // JWS ES256 requires the RAW, fixed-length 64-byte r||s
    // concatenation instead. Converts one to the other.
    private fun derToRawEcdsaSignature(der: ByteArray): ByteArray {
        // SEQUENCE tag(1) + length(1-2) + INTEGER tag(1) + len(1) + r + INTEGER tag(1) + len(1) + s
        var offset = 2 // skip SEQUENCE tag + length byte (assumes short-form length, true for P-256 sigs)
        if ((der[1].toInt() and 0x80) != 0) offset += (der[1].toInt() and 0x7F) // long-form length, rare but real
        fun readInt(off: Int): Pair<ByteArray, Int> {
            require(der[off] == 0x02.toByte()) { "expected INTEGER tag in DER signature" }
            val len = der[off + 1].toInt() and 0xFF
            val start = off + 2
            return der.copyOfRange(start, start + len) to (start + len)
        }
        val (rBytes, afterR) = readInt(offset)
        val (sBytes, _) = readInt(afterR)
        fun trimTo32(b: ByteArray): ByteArray {
            val t = if (b.size > 32 && b[0].toInt() == 0) b.copyOfRange(1, b.size) else b
            return if (t.size == 32) t else ByteArray(32 - t.size) + t
        }
        return trimTo32(rBytes) + trimTo32(sBytes)
    }

    /**
     * Builds and signs one DPoP proof JWT (RFC 9449 section 4.2).
     * `nonce` is omitted on the first attempt at a given endpoint; the
     * authorization/resource server commonly responds `400
     * use_dpop_nonce` with a `DPoP-Nonce` response header on that first
     * try in practice (a real atproto behavior, not hypothetical) --
     * the caller retries once with that nonce included here.
     * `accessToken`, when present, adds the `ath` claim (its SHA-256
     * hash) -- required when this proof accompanies an already-
     * DPoP-bound resource-server request, omitted for the token
     * endpoint's own initial exchange.
     */
    fun createProof(htm: String, htu: String, nonce: String? = null, accessToken: String? = null): String {
        val header = JSONObject()
            .put("typ", "dpop+jwt")
            .put("alg", "ES256")
            .put("jwk", publicJwk())
        val payload = JSONObject()
            .put("jti", UUID.randomUUID().toString())
            .put("htm", htm)
            .put("htu", htu)
            .put("iat", System.currentTimeMillis() / 1000)
        if (nonce != null) payload.put("nonce", nonce)
        if (accessToken != null) {
            val ath = MessageDigest.getInstance("SHA-256").digest(accessToken.toByteArray(Charsets.UTF_8))
            payload.put("ath", b64url(ath))
        }

        val signingInput = "${b64url(header.toString().toByteArray(Charsets.UTF_8))}.${b64url(payload.toString().toByteArray(Charsets.UTF_8))}"
        // AndroidKeyStore's private-key handle is opaque -- it only
        // implements java.security.PrivateKey (Signature.initSign's
        // real parameter type), not java.security.interfaces.ECPrivateKey.
        // A cast to the latter throws ClassCastException at runtime,
        // confirmed on-device, not hypothetical -- there's no actual
        // need for the narrower type here anyway.
        val sig = Signature.getInstance("SHA256withECDSA").apply {
            initSign(privateKeyEntry().privateKey)
            update(signingInput.toByteArray(Charsets.UTF_8))
        }.sign()
        return "$signingInput.${b64url(derToRawEcdsaSignature(sig))}"
    }
}
