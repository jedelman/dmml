#include <jni.h>
#include <stdio.h>

int hs_spike_embed_jvm_and_find_string_class(void) {
    JavaVM *jvm;
    JNIEnv *env;
    JavaVMInitArgs vm_args;
    JavaVMOption options[1];
    options[0].optionString = "-Xrs";
    vm_args.version = JNI_VERSION_21;
    vm_args.nOptions = 1;
    vm_args.options = options;
    vm_args.ignoreUnrecognized = JNI_FALSE;

    jint rc = JNI_CreateJavaVM(&jvm, (void**)&env, &vm_args);
    if (rc != JNI_OK) {
        fprintf(stderr, "shim: JNI_CreateJavaVM failed: %d\n", rc);
        return 1;
    }
    jclass stringClass = (*env)->FindClass(env, "java/lang/String");
    if (stringClass == NULL) {
        fprintf(stderr, "shim: FindClass failed\n");
        (*jvm)->DestroyJavaVM(jvm);
        return 1;
    }
    fprintf(stderr, "shim: FindClass(java/lang/String) OK from inside GHC RTS process\n");
    (*jvm)->DestroyJavaVM(jvm);
    return 0;
}
