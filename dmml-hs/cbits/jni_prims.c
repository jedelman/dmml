// Generic, reusable JNI primitives -- deliberately NOT one function per
// JGit method. DMML.Jgit's typed Haskell wrappers hold each JGit method's
// exact signature string exactly once, right beside its Haskell type
// signature; this file supplies only the small, fixed set of JNI
// operation *shapes* (find a class, look up a method by arity/return
// kind, call it, marshal a String) that every one of those wrappers is
// built from. See written-world/dev-journal/2026-09-07-jgit-canonical-
// single-implementation.md for why this exists at all (one canonical
// Haskell+JGit implementation for CLI and Android) and dmml-hs/spikes/
// jvm-embed/ for the real spike that proved a GHC-compiled binary can
// embed a JVM via this same Invocation API in the first place.

#include <jni.h>
#include <stdlib.h>
#include <string.h>

// --- VM lifecycle -----------------------------------------------------
//
// Desktop/CLI only: this embeds a fresh JVM via JNI_CreateJavaVM. The
// Android side of "one canonical implementation" is the OPPOSITE
// direction -- an upcall into the JVM Kotlin already started, no
// JNI_CreateJavaVM there at all -- and is NOT this file's concern; it
// needs its own entry point on that platform, not yet written.
JNIEnv *hs_jgit_create_jvm(JavaVM **out_jvm, const char *classpath) {
    JavaVM *jvm;
    JNIEnv *env;
    JavaVMInitArgs vm_args;
    JavaVMOption options[1];
    char *opt = malloc(strlen(classpath) + 32);
    sprintf(opt, "-Djava.class.path=%s", classpath);
    options[0].optionString = opt;
    vm_args.version = JNI_VERSION_21;
    vm_args.nOptions = 1;
    vm_args.options = options;
    vm_args.ignoreUnrecognized = JNI_FALSE;

    jint rc = JNI_CreateJavaVM(&jvm, (void **)&env, &vm_args);
    free(opt);
    if (rc != JNI_OK) return NULL;
    *out_jvm = jvm;
    return env;
}

void hs_jgit_destroy_jvm(JavaVM *jvm) {
    if (jvm != NULL) (*jvm)->DestroyJavaVM(jvm);
}

// --- Class/method lookup -----------------------------------------------

jclass hs_jni_find_class(JNIEnv *env, const char *name) {
    jclass c = (*env)->FindClass(env, name);
    return c;
}

jmethodID hs_jni_get_method_id(JNIEnv *env, jclass cls, const char *name, const char *sig) {
    return (*env)->GetMethodID(env, cls, name, sig);
}

jmethodID hs_jni_get_static_method_id(JNIEnv *env, jclass cls, const char *name, const char *sig) {
    return (*env)->GetStaticMethodID(env, cls, name, sig);
}

// --- Calls, by shape (arity + arg/return kind), not by JGit method -----
// Every one of these is a thin, generic wrapper over the real JNIEnv
// function-pointer table -- GHC's FFI can't call through that table
// directly, hence this file, but nothing here is specific to any one
// JGit call.

jobject hs_jni_new_object_1obj(JNIEnv *env, jclass cls, jmethodID ctor, jobject arg0) {
    return (*env)->NewObject(env, cls, ctor, arg0);
}

jobject hs_jni_call_static_object_method_0(JNIEnv *env, jclass cls, jmethodID m) {
    return (*env)->CallStaticObjectMethod(env, cls, m);
}

jobject hs_jni_call_static_object_method_1obj(JNIEnv *env, jclass cls, jmethodID m, jobject arg0) {
    return (*env)->CallStaticObjectMethod(env, cls, m, arg0);
}

jobject hs_jni_call_static_object_method_1bool(JNIEnv *env, jclass cls, jmethodID m, jboolean arg0) {
    return (*env)->CallStaticObjectMethod(env, cls, m, arg0);
}

jobject hs_jni_call_object_method_0(JNIEnv *env, jobject recv, jmethodID m) {
    return (*env)->CallObjectMethod(env, recv, m);
}

jobject hs_jni_call_object_method_1obj(JNIEnv *env, jobject recv, jmethodID m, jobject arg0) {
    return (*env)->CallObjectMethod(env, recv, m, arg0);
}

jobject hs_jni_call_object_method_1str(JNIEnv *env, jobject recv, jmethodID m, jstring arg0) {
    return (*env)->CallObjectMethod(env, recv, m, arg0);
}

jobject hs_jni_call_object_method_1bool(JNIEnv *env, jobject recv, jmethodID m, jboolean arg0) {
    return (*env)->CallObjectMethod(env, recv, m, arg0);
}

// --- String marshaling ---------------------------------------------------

jstring hs_jni_new_string_utf(JNIEnv *env, const char *s) {
    return (*env)->NewStringUTF(env, s);
}

// Caller must free() the returned buffer.
char *hs_jni_get_string_utf_chars_copy(JNIEnv *env, jstring s) {
    const char *chars = (*env)->GetStringUTFChars(env, s, NULL);
    if (chars == NULL) return NULL;
    char *copy = strdup(chars);
    (*env)->ReleaseStringUTFChars(env, s, chars);
    return copy;
}

// --- Exception handling --------------------------------------------------
// Every call site checks this after a call that could throw -- a JGit
// GitAPIException surfaces here, not as a C return code.

jboolean hs_jni_exception_check(JNIEnv *env) {
    return (*env)->ExceptionCheck(env);
}

// Returns a malloc'd string describing the pending exception (its class
// name + message via toString()), clears it, or NULL if none/failed.
char *hs_jni_describe_and_clear_exception(JNIEnv *env) {
    if (!(*env)->ExceptionCheck(env)) return NULL;
    jthrowable exc = (*env)->ExceptionOccurred(env);
    (*env)->ExceptionClear(env);
    if (exc == NULL) return strdup("(unknown exception, ExceptionOccurred returned null)");

    jclass throwableCls = (*env)->FindClass(env, "java/lang/Throwable");
    if (throwableCls == NULL) return strdup("(exception occurred; could not find java/lang/Throwable to describe it)");
    jmethodID toStringM = (*env)->GetMethodID(env, throwableCls, "toString", "()Ljava/lang/String;");
    if (toStringM == NULL) return strdup("(exception occurred; could not find Throwable.toString())");
    jstring desc = (jstring)(*env)->CallObjectMethod(env, exc, toStringM);
    if (desc == NULL) return strdup("(exception occurred; toString() returned null)");
    return hs_jni_get_string_utf_chars_copy(env, desc);
}
