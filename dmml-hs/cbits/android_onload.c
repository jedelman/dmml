// Android JNI_OnLoad shim -- binds DMML.AndroidBridge's foreign-
// exported Haskell functions to real Kotlin `external fun` native
// methods via RegisterNatives, rather than relying on
// Java_pkg_Class_method name-mangling (which would require this file
// to hardcode a specific Android app package/class name that didn't
// exist yet when this was written -- see DMML.AndroidBridge's own doc
// comment). A real Android app is free to point the class name below
// at wherever it actually declares these `external fun`s; nothing
// else here needs to change.
//
// DMML.AndroidBridge's exported functions all take and return HsPtr
// (== void*) -- confirmed directly from the real GHC-generated stub
// header (DMML_AndroidBridge_stub.h, produced by `ghc -stubdir` when
// AndroidBridge.hs is compiled -- included below rather than
// hand-duplicating the signatures, so this file can never drift from
// what GHC actually generates), not by assumption. Every argument
// after the first is a plain, NUL-terminated `char*` in practice, and
// every return is a malloc'd `char*` GHC's own `newCString` produced
// (see DMML.AndroidBridge's `marshalStr`/`marshalText`) -- this file's
// job is exactly the hop between that convention and real JNI
// `jstring`s, nothing more.
//
// NOT YET VERIFIED ON A REAL DEVICE/EMULATOR: written to match the
// real jni.h this project already builds against on desktop (the
// JNI C API itself is portable; only the final NDK cross-compile and
// an actual RegisterNatives call inside a running Android process are
// untested here) -- see the handoff to the laptop agent for what
// still needs a real AVD to confirm.

#include <jni.h>
#include <stdlib.h>
#include <string.h>
#include <HsFFI.h>
#include "DMML/AndroidBridge_stub.h"

// Real Android app: change this if the app's package/class for the
// native methods differs. Everything else in this file is generic.
#define BRIDGE_CLASS "org/jasonedelman/writtenworld/NativeBridge"

static char *jstring_to_cstr(JNIEnv *env, jstring s) {
    const char *chars = (*env)->GetStringUTFChars(env, s, NULL);
    if (chars == NULL) return NULL;
    char *copy = strdup(chars);
    (*env)->ReleaseStringUTFChars(env, s, chars);
    return copy;
}

// Wraps a Haskell-returned malloc'd char* (from newCString) into a
// real jstring and frees the original -- every native_* wrapper below
// does this exactly once, right before returning.
static jstring take_hs_cstring(JNIEnv *env, void *hsResult) {
    char *result = (char *)hsResult;
    jstring jresult = (*env)->NewStringUTF(env, result ? result : "ERROR: null result from Haskell");
    if (result != NULL) free(result);
    return jresult;
}

static jstring native_jgitCommit(JNIEnv *env, jclass clazz, jstring repoDir, jstring relPath, jstring content, jstring message) {
    (void)clazz;
    char *c_repoDir = jstring_to_cstr(env, repoDir);
    char *c_relPath = jstring_to_cstr(env, relPath);
    char *c_content = jstring_to_cstr(env, content);
    char *c_message = jstring_to_cstr(env, message);
    void *result = android_jgit_commit(env, c_repoDir, c_relPath, c_content, c_message);
    free(c_repoDir); free(c_relPath); free(c_content); free(c_message);
    return take_hs_cstring(env, result);
}

static jstring native_atprotoResolve(JNIEnv *env, jclass clazz, jstring identifier) {
    (void)clazz;
    char *c_identifier = jstring_to_cstr(env, identifier);
    void *result = android_atproto_resolve(env, c_identifier);
    free(c_identifier);
    return take_hs_cstring(env, result);
}

static jstring native_atprotoPull(JNIEnv *env, jclass clazz, jstring peer, jstring collection, jstring cursor) {
    (void)clazz;
    char *c_peer = jstring_to_cstr(env, peer);
    char *c_collection = jstring_to_cstr(env, collection);
    char *c_cursor = jstring_to_cstr(env, cursor);
    void *result = android_atproto_pull(env, c_peer, c_collection, c_cursor);
    free(c_peer); free(c_collection); free(c_cursor);
    return take_hs_cstring(env, result);
}

static jstring native_atprotoCreateSession(JNIEnv *env, jclass clazz, jstring pdsEndpoint, jstring identifier, jstring password) {
    (void)clazz;
    char *c_pdsEndpoint = jstring_to_cstr(env, pdsEndpoint);
    char *c_identifier = jstring_to_cstr(env, identifier);
    char *c_password = jstring_to_cstr(env, password);
    void *result = android_atproto_create_session(env, c_pdsEndpoint, c_identifier, c_password);
    free(c_pdsEndpoint); free(c_identifier); free(c_password);
    return take_hs_cstring(env, result);
}

static jstring native_atprotoCreateRecord(JNIEnv *env, jclass clazz, jstring pdsEndpoint, jstring did, jstring accessJwt, jstring collection, jstring predicate, jstring dmmlText) {
    (void)clazz;
    char *c_pdsEndpoint = jstring_to_cstr(env, pdsEndpoint);
    char *c_did = jstring_to_cstr(env, did);
    char *c_accessJwt = jstring_to_cstr(env, accessJwt);
    char *c_collection = jstring_to_cstr(env, collection);
    char *c_predicate = jstring_to_cstr(env, predicate);
    char *c_dmmlText = jstring_to_cstr(env, dmmlText);
    void *result = android_atproto_create_record(env, c_pdsEndpoint, c_did, c_accessJwt, c_collection, c_predicate, c_dmmlText);
    free(c_pdsEndpoint); free(c_did); free(c_accessJwt); free(c_collection); free(c_predicate); free(c_dmmlText);
    return take_hs_cstring(env, result);
}

static jstring native_atprotoCreateRecordDpop(JNIEnv *env, jclass clazz, jstring pdsEndpoint, jstring did, jstring accessToken, jstring collection, jstring predicate, jstring dmmlText) {
    (void)clazz;
    char *c_pdsEndpoint = jstring_to_cstr(env, pdsEndpoint);
    char *c_did = jstring_to_cstr(env, did);
    char *c_accessToken = jstring_to_cstr(env, accessToken);
    char *c_collection = jstring_to_cstr(env, collection);
    char *c_predicate = jstring_to_cstr(env, predicate);
    char *c_dmmlText = jstring_to_cstr(env, dmmlText);
    void *result = android_atproto_create_record_dpop(env, c_pdsEndpoint, c_did, c_accessToken, c_collection, c_predicate, c_dmmlText);
    free(c_pdsEndpoint); free(c_did); free(c_accessToken); free(c_collection); free(c_predicate); free(c_dmmlText);
    return take_hs_cstring(env, result);
}

static jstring native_llmChatComplete(JNIEnv *env, jclass clazz, jstring apiKey, jstring model, jstring systemPrompt, jstring userPrompt) {
    (void)clazz;
    char *c_apiKey = jstring_to_cstr(env, apiKey);
    char *c_model = jstring_to_cstr(env, model);
    char *c_systemPrompt = jstring_to_cstr(env, systemPrompt);
    char *c_userPrompt = jstring_to_cstr(env, userPrompt);
    void *result = android_llm_chat_complete(env, c_apiKey, c_model, c_systemPrompt, c_userPrompt);
    free(c_apiKey); free(c_model); free(c_systemPrompt); free(c_userPrompt);
    return take_hs_cstring(env, result);
}

static jstring native_brokerIncorporate(JNIEnv *env, jclass clazz, jstring repoDir, jstring peerIdentifier, jstring cursorFile, jstring commitsDir) {
    (void)clazz;
    char *c_repoDir = jstring_to_cstr(env, repoDir);
    char *c_peerIdentifier = jstring_to_cstr(env, peerIdentifier);
    char *c_cursorFile = jstring_to_cstr(env, cursorFile);
    char *c_commitsDir = jstring_to_cstr(env, commitsDir);
    void *result = android_broker_incorporate(env, c_repoDir, c_peerIdentifier, c_cursorFile, c_commitsDir);
    free(c_repoDir); free(c_peerIdentifier); free(c_cursorFile); free(c_commitsDir);
    return take_hs_cstring(env, result);
}

static JNINativeMethod bridgeMethods[] = {
    {"jgitCommit", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_jgitCommit},
    {"atprotoResolve", "(Ljava/lang/String;)Ljava/lang/String;", (void *)native_atprotoResolve},
    {"atprotoPull", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_atprotoPull},
    {"atprotoCreateSession", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_atprotoCreateSession},
    {"atprotoCreateRecord", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_atprotoCreateRecord},
    {"atprotoCreateRecordDpop", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_atprotoCreateRecordDpop},
    {"llmChatComplete", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_llmChatComplete},
    {"brokerIncorporate", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", (void *)native_brokerIncorporate},
};

// Called automatically by the JVM the instant this .so is loaded
// (Kotlin's System.loadLibrary() on Android; plain System.load() in
// the desktop-JDK verification harness this was proven against, since
// the JNI mechanism itself -- OnLoad, RegisterNatives, the calling
// convention -- is identical on both, only the final NDK cross-compile
// isn't done here). Also where the Haskell RTS itself gets started
// (hs_init) -- nothing else calls it for a .so loaded INTO an
// already-running JVM, unlike a Haskell-main-having desktop executable
// where GHC's own generated main() calls it automatically. Skipping
// this was a real gap caught before ever linking a real .so: every
// android_* call below would have run against an uninitialized RTS.
JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    (void)reserved;
    JNIEnv *env;
    if ((*vm)->GetEnv(vm, (void **)&env, JNI_VERSION_1_6) != JNI_OK) {
        return JNI_ERR;
    }
    { int argc = 1; char *argv[] = {"dmml-android-bridge", NULL}; char **pargv = argv;
      hs_init(&argc, &pargv); }
    jclass clazz = (*env)->FindClass(env, BRIDGE_CLASS);
    if (clazz == NULL) {
        return JNI_ERR;
    }
    if ((*env)->RegisterNatives(env, clazz, bridgeMethods, sizeof(bridgeMethods) / sizeof(bridgeMethods[0])) != JNI_OK) {
        return JNI_ERR;
    }
    return JNI_VERSION_1_6;
}

// Mirror of JNI_OnLoad -- shuts the RTS down cleanly when the JVM
// unloads this library. Not calling this wouldn't crash anything in
// practice (the process is going away too, on Android's case), but a
// clean hs_exit is the correct symmetric counterpart and costs nothing.
JNIEXPORT void JNICALL JNI_OnUnload(JavaVM *vm, void *reserved) {
    (void)vm;
    (void)reserved;
    hs_exit();
}
