{-# LANGUAGE OverloadedStrings #-}

-- | Host-side verification of 'DMML.JniBridge', in the same spirit as
-- android-poc/README.md's own "verified for real, on host GHC" section
-- for 'hsGreet': calls the EXACT exported 'CString' functions
-- (@dmml_render@\/@dmml_actions@\/@dmml_fire@), through real
-- 'Foreign.C.String' marshaling, not their pure Haskell cores directly
-- -- proving the whole FFI-shaped surface a JNI caller would actually
-- use, before any JNI\/Android toolchain is involved at all. What this
-- does NOT prove: the JNI C shim (jni_bridge.c) or a real Android
-- cross-compile -- see android-poc/README.md and dmml/dev-journal/
-- 2026-09-04-android-jni-vs-ipc.md for what's still open there.
module Main (main) where

import Data.Aeson (encode)
import qualified Data.Text as T
import qualified Data.Text.Lazy as TL
import qualified Data.Text.Lazy.Encoding as TLE
import Foreign.C.String (newCString, peekCString)
import System.Exit (exitFailure)

import DMML.JniBridge (dmml_actions, dmml_actions_history, dmml_fire, dmml_fire_history, dmml_render, dmml_render_history)

worldSrc :: String
worldSrc =
  unlines
    [ "commit prospect"
    , "  declare relation location"
    , "  declare relation state"
    , ""
    , "  player/one `location` room/forge"
    , "  player/one `state` idle"
    ]

machineSrc :: String
machineSrc =
  unlines
    [ "machine machine/actions"
    , ""
    , "  states"
    , "    idle"
    , "    working"
    , ""
    , "  transition work()"
    , "    idle -> working"
    , "    guard self `location` room/forge"
    , "    assert working"
    , "    retract idle"
    , ""
    , "  transition rest()"
    , "    working -> idle"
    , "    guard self `location` room/forge"
    , "    assert idle"
    , "    retract working"
    ]

selfNode :: String
selfNode = "player/one"

check :: String -> Bool -> IO ()
check label ok =
  putStrLn ((if ok then "OK   " else "FAIL ") <> label) >> if ok then pure () else exitFailure

main :: IO ()
main = do
  worldC <- newCString worldSrc
  machineC <- newCString machineSrc
  selfC <- newCString selfNode
  transC <- newCString "work"
  emptyC <- newCString ""

  putStrLn "=== dmml_render ==="
  renderedC <- dmml_render worldC machineC
  rendered <- peekCString renderedC
  putStrLn rendered
  check "render succeeded (no ERROR: prefix)" (take 6 rendered /= "ERROR:")
  check "render mentions player/one" ("player/one" `isInfixOfStr` rendered)

  putStrLn ""
  putStrLn "=== dmml_actions (before firing) ==="
  actionsBeforeC <- dmml_actions worldC machineC selfC
  actionsBefore <- peekCString actionsBeforeC
  putStrLn actionsBefore
  check "actions succeeded" (take 6 actionsBefore /= "ERROR:")
  check "work() is legal before firing" ("machine/actions/work" `isInfixOfStr` actionsBefore)

  putStrLn ""
  putStrLn "=== dmml_fire (work) ==="
  firedC <- dmml_fire worldC machineC selfC transC
  fired <- peekCString firedC
  putStrLn fired
  check "fire succeeded (real commit, not ERROR:)" (take 6 fired /= "ERROR:")
  check "fired commit asserts working" ("working" `isInfixOfStr` fired)
  check "fired commit consumes idle's real provenance" ("consumes" `isInfixOfStr` fired && "fnv1a64:" `isInfixOfStr` fired)

  putStrLn ""
  putStrLn "=== error path: unknown transition ==="
  badTransC <- newCString "nosuchtransition"
  badC <- dmml_fire worldC machineC selfC badTransC
  bad <- peekCString badC
  putStrLn bad
  check "unknown transition reports ERROR:, doesn't crash" (take 6 bad == "ERROR:")

  putStrLn ""
  putStrLn "=== error path: malformed world source ==="
  garbageC <- newCString "this is not dmml at all {{{"
  brokenC <- dmml_render garbageC emptyC
  broken <- peekCString brokenC
  putStrLn (take 120 broken)
  check "malformed input reports ERROR:, doesn't crash" (take 6 broken == "ERROR:")

  putStrLn ""
  putStrLn "=== history mode: dmml_fire_history across two real rounds ==="
  -- Same round-trip a Compose UI actually does: keep a growing
  -- [Text] history on the CALLER's side, JSON-encode it (Data.Aeson,
  -- the same library DMML.JniBridge decodes it with), fire against it,
  -- append the newly-fired commit, repeat -- proving the history
  -- variants added for the Compose UI actually chain across multiple
  -- real fires, not just one.
  let jsonHistory h = TL.unpack (TLE.decodeUtf8 (encode (map T.pack h)))
  history0C <- newCString (jsonHistory [worldSrc])
  actionsH0C <- dmml_actions_history history0C machineC selfC
  actionsH0 <- peekCString actionsH0C
  check "history actions (round 0) offers work()" ("machine/actions/work" `isInfixOfStr` actionsH0)

  fire1C <- dmml_fire_history history0C machineC selfC transC
  fire1 <- peekCString fire1C
  check "history fire (round 0->1) succeeded" (take 6 fire1 /= "ERROR:")

  history1C <- newCString (jsonHistory [worldSrc, fire1])
  actionsH1C <- dmml_actions_history history1C machineC selfC
  actionsH1 <- peekCString actionsH1C
  check "history actions (round 1) now offers rest(), not work()" ("machine/actions/rest" `isInfixOfStr` actionsH1 && not ("machine/actions/work" `isInfixOfStr` actionsH1))

  restTransC <- newCString "rest"
  fire2C <- dmml_fire_history history1C machineC selfC restTransC
  fire2 <- peekCString fire2C
  check "history fire (round 1->2) succeeded" (take 6 fire2 /= "ERROR:")
  check "second fire cites the FIRST fire's provenance, not the original world's" ("android:history1" `isInfixOfStr` fire2)

  history2C <- newCString (jsonHistory [worldSrc, fire1, fire2])
  renderedH2C <- dmml_render_history history2C machineC
  renderedH2 <- peekCString renderedH2C
  check "final rendered state is idle again (toggled twice)" ("state = idle" `isInfixOfStr` renderedH2)

  putStrLn ""
  putStrLn "all checks passed"
  where
    isInfixOfStr needle haystack = go haystack
      where
        n = length needle
        go [] = null needle
        go s@(_ : rest) = take n s == needle || go rest
