{-# LANGUAGE ForeignFunctionInterface #-}
module Main where

import Foreign.C.Types (CInt(..))

foreign import ccall safe "hs_spike_embed_jvm_and_find_string_class"
  c_embedJvm :: IO CInt

main :: IO ()
main = do
  putStrLn "Haskell RTS: about to embed a JVM via JNI Invocation API..."
  rc <- c_embedJvm
  if rc == 0
    then putStrLn "Haskell RTS: JVM embed + FindClass + destroy succeeded"
    else putStrLn ("Haskell RTS: FAILED, rc=" ++ show rc)
