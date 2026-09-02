{-# LANGUAGE ForeignFunctionInterface #-}

-- | F1 (jedelman/dmml#1): the smallest real thing that proves a Haskell
-- function can be called across a JNI boundary at all, before
-- committing dmml-hs's actual interpreter to the Android platform
-- pivot (written-world/dev-journal/2026-09-02-platform-pivot-cli-
-- android-filesystem-canonical.md). Deliberately NOT the interpreter
-- itself -- proving the bridge mechanism is a separate, prior question
-- from proving the interpreter works once it's on the other side of
-- that bridge, and conflating them would make a failure here
-- ambiguous about which one broke.
--
-- `foreign export ccall` is what makes 'hsGreet' callable as a plain C
-- symbol (`hsGreet`) from jni/jni_bridge.c -- GHC generates that symbol
-- plus a matching `Bridge_stub.h` header at compile time from this
-- declaration alone, no separate binding-generator step.
module Bridge (hsGreet) where

import Foreign.C.String (CString, newCString)

foreign export ccall hsGreet :: IO CString

-- | Returns a heap-allocated C string the caller owns and must free
-- with plain C `free()` -- 'newCString' allocates via the C allocator
-- (not GHC's own managed heap), which is what makes that safe and
-- correct from the C/JNI side; see jni_bridge.c's own comment at the
-- call site.
hsGreet :: IO CString
hsGreet = newCString "hello from the GHC RTS, via JNI"
