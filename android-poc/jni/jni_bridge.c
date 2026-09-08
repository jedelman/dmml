// The real Android JNI bridge, replacing F1's fake `hsGreet` PoC now
// that dmml/dev-journal/2026-09-04-android-jni-vs-ipc.md's recommendation
// has a real module behind it: DMML.JniBridge (dmml-hs/src/DMML/
// JniBridge.hs), calling dmml-hs's own DMML.Materialize/DMML.Guard/
// DMML.Fire directly -- NOT this CLI's argv interface wrapped a second
// time. Verified end-to-end on host GHC via `cabal run
// jni-bridge-smoke-test` (dmml-hs/app/JniBridgeSmokeTest.hs) BEFORE this
// file was written -- see dev-journal/2026-09-06-real-jni-bridge-
// verified-on-host.md for exactly what that proved and what it didn't.
//
// NOT YET VERIFIED: this file itself, cross-compiled and linked against
// a real Android NDK + cross GHC, or run on a real device/emulator --
// same disclosed gap android-poc/README.md's original PoC had, now one
// layer further in (the interpreter side is real; the JNI plumbing
// around it still isn't proven on-device).
//
// RTS lifecycle: same deliberate non-answer as the original PoC --
// `hs_exit()` is never called. Still out of scope here; a real app needs
// a real decision, not a silent assumption.

#include <jni.h>
#include <HsFFI.h>
#include <stdlib.h>
#include "DMML/JniBridge_stub.h"

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    int argc = 1;
    char *argv[] = {"dmmlbridge", NULL};
    char **pargv = argv;
    hs_init(&argc, &pargv);
    return JNI_VERSION_1_6;
}

// Shared shape every native method below follows: pull each jstring
// argument into a plain C string (GetStringUTFChars), call straight into
// the corresponding `dmml_*` symbol DMML.JniBridge's `foreign export
// ccall` declarations generate, wrap the result in a jstring, then free
// BOTH the JNI-owned UTF chars (ReleaseStringUTFChars) and the
// GHC-C-allocator-owned result string (plain C free() -- correct here
// for the same reason it was correct in the original PoC: Foreign.C.
// String.newCString allocates via the C allocator, not GHC's managed
// heap; see DMML.JniBridge's own module header).
//
// Class name: org.writtenworld.androidpoc.DmmlBridge (NOT MainActivity
// -- the real interpreter boundary is deliberately kept separate from
// any one Activity's lifecycle, so it can be called from wherever the
// real UI layer ends up living).

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_render(JNIEnv *env, jobject thiz, jstring worldSrc, jstring machineSrc) {
    const char *world = (*env)->GetStringUTFChars(env, worldSrc, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineSrc, NULL);

    char *hsResult = dmml_render((char *)world, (char *)machine);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, worldSrc, world);
    (*env)->ReleaseStringUTFChars(env, machineSrc, machine);
    return result;
}

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_actions(JNIEnv *env, jobject thiz, jstring worldSrc, jstring machineSrc, jstring selfNode) {
    const char *world = (*env)->GetStringUTFChars(env, worldSrc, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineSrc, NULL);
    const char *self = (*env)->GetStringUTFChars(env, selfNode, NULL);

    char *hsResult = dmml_actions((char *)world, (char *)machine, (char *)self);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, worldSrc, world);
    (*env)->ReleaseStringUTFChars(env, machineSrc, machine);
    (*env)->ReleaseStringUTFChars(env, selfNode, self);
    return result;
}

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_fire(JNIEnv *env, jobject thiz, jstring worldSrc, jstring machineSrc, jstring selfNode, jstring transitionIdent) {
    const char *world = (*env)->GetStringUTFChars(env, worldSrc, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineSrc, NULL);
    const char *self = (*env)->GetStringUTFChars(env, selfNode, NULL);
    const char *transition = (*env)->GetStringUTFChars(env, transitionIdent, NULL);

    char *hsResult = dmml_fire((char *)world, (char *)machine, (char *)self, (char *)transition);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, worldSrc, world);
    (*env)->ReleaseStringUTFChars(env, machineSrc, machine);
    (*env)->ReleaseStringUTFChars(env, selfNode, self);
    (*env)->ReleaseStringUTFChars(env, transitionIdent, transition);
    return result;
}

// History-aware variants (added 2026-09-06 for the Compose UI, see
// DMML.JniBridge.hs's own header comment for why the single-commit
// functions above can't support more than one fire). `worldHistoryJson`
// is a plain JSON string (a JSON array of world-commit strings) -- no
// different in kind from any other jstring argument here, decoded on
// the Haskell side via Data.Aeson, not by this file.

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_renderHistory(JNIEnv *env, jobject thiz, jstring worldHistoryJson, jstring machineSrc) {
    const char *history = (*env)->GetStringUTFChars(env, worldHistoryJson, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineSrc, NULL);

    char *hsResult = dmml_render_history((char *)history, (char *)machine);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, worldHistoryJson, history);
    (*env)->ReleaseStringUTFChars(env, machineSrc, machine);
    return result;
}

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_actionsHistory(JNIEnv *env, jobject thiz, jstring worldHistoryJson, jstring machineSrc, jstring selfNode) {
    const char *history = (*env)->GetStringUTFChars(env, worldHistoryJson, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineSrc, NULL);
    const char *self = (*env)->GetStringUTFChars(env, selfNode, NULL);

    char *hsResult = dmml_actions_history((char *)history, (char *)machine, (char *)self);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, worldHistoryJson, history);
    (*env)->ReleaseStringUTFChars(env, machineSrc, machine);
    (*env)->ReleaseStringUTFChars(env, selfNode, self);
    return result;
}

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_fireHistory(JNIEnv *env, jobject thiz, jstring worldHistoryJson, jstring machineSrc, jstring selfNode, jstring transitionIdent) {
    const char *history = (*env)->GetStringUTFChars(env, worldHistoryJson, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineSrc, NULL);
    const char *self = (*env)->GetStringUTFChars(env, selfNode, NULL);
    const char *transition = (*env)->GetStringUTFChars(env, transitionIdent, NULL);

    char *hsResult = dmml_fire_history((char *)history, (char *)machine, (char *)self, (char *)transition);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, worldHistoryJson, history);
    (*env)->ReleaseStringUTFChars(env, machineSrc, machine);
    (*env)->ReleaseStringUTFChars(env, selfNode, self);
    (*env)->ReleaseStringUTFChars(env, transitionIdent, transition);
    return result;
}

// Directory-aware variants (added 2026-09-07 alongside WorldRepository.kt
// and DMML.Loader -- see dmml/dev-journal/2026-09-07-android-jgit-sync-
// spec.md). `dirPath` is a real filesystem path; DMML.Loader reads
// *.dmml files from it directly on the Haskell side, no JSON
// serialization of file contents across this boundary at all.

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_renderDir(JNIEnv *env, jobject thiz, jstring dirPath) {
    const char *dir = (*env)->GetStringUTFChars(env, dirPath, NULL);

    char *hsResult = dmml_render_dir((char *)dir);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, dirPath, dir);
    return result;
}

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_actionsDir(JNIEnv *env, jobject thiz, jstring dirPath, jstring selfNode) {
    const char *dir = (*env)->GetStringUTFChars(env, dirPath, NULL);
    const char *self = (*env)->GetStringUTFChars(env, selfNode, NULL);

    char *hsResult = dmml_actions_dir((char *)dir, (char *)self);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, dirPath, dir);
    (*env)->ReleaseStringUTFChars(env, selfNode, self);
    return result;
}

JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_DmmlBridge_fireDir(JNIEnv *env, jobject thiz, jstring dirPath, jstring selfNode, jstring machineNode, jstring transitionIdent) {
    const char *dir = (*env)->GetStringUTFChars(env, dirPath, NULL);
    const char *self = (*env)->GetStringUTFChars(env, selfNode, NULL);
    const char *machine = (*env)->GetStringUTFChars(env, machineNode, NULL);
    const char *transition = (*env)->GetStringUTFChars(env, transitionIdent, NULL);

    char *hsResult = dmml_fire_dir((char *)dir, (char *)self, (char *)machine, (char *)transition);
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);

    (*env)->ReleaseStringUTFChars(env, dirPath, dir);
    (*env)->ReleaseStringUTFChars(env, selfNode, self);
    (*env)->ReleaseStringUTFChars(env, machineNode, machine);
    (*env)->ReleaseStringUTFChars(env, transitionIdent, transition);
    return result;
}
