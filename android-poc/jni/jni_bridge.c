// F1 (jedelman/dmml#1): the JNI half of the bridge. Loaded by
// MainActivity's `System.loadLibrary("dmmlbridge")`; boots the GHC RTS
// once (JNI_OnLoad, not lazily on first call -- so a failure to start
// the RTS surfaces immediately at library-load time, not on whatever
// call happens to be first) and exposes one native method that calls
// straight into Haskell.
//
// The RTS-lifecycle question this file answers by NOT answering it:
// `hs_exit()` is never called. For a real app this needs a real
// decision (call it from a JNI_OnUnload that Android may not reliably
// invoke, or accept the RTS lives exactly as long as the process does)
// -- deliberately out of scope for a bridge-mechanism proof-of-concept,
// noted here rather than silently assumed.

#include <jni.h>
#include <HsFFI.h>
#include <stdlib.h>
#include "Bridge_stub.h"

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    int argc = 1;
    char *argv[] = {"dmmlbridge", NULL};
    char **pargv = argv;
    hs_init(&argc, &pargv);
    return JNI_VERSION_1_6;
}

// Name is JNI's own mangling convention: Java_<package_with_underscores>_<Class>_<method>.
// org.writtenworld.androidpoc.MainActivity.greetFromHaskell()
JNIEXPORT jstring JNICALL
Java_org_writtenworld_androidpoc_MainActivity_greetFromHaskell(JNIEnv *env, jobject thiz) {
    // hsGreet allocates via the C allocator (Foreign.C.String.newCString
    // does NOT use GHC's managed heap) -- freeing it with plain C
    // free() here is correct, not a leak/double-free risk, and matches
    // the same round-trip already verified natively in
    // android-poc/README.md's own verification section.
    char *hsResult = hsGreet();
    jstring result = (*env)->NewStringUTF(env, hsResult);
    free(hsResult);
    return result;
}
