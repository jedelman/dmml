{-# LANGUAGE OverloadedStrings #-}

-- | A real interactive frontend for the CLI, built on the exact same
-- primitives 'DMML.JniBridge' verified end-to-end
-- (jni-bridge-smoke-test) and android-poc/'s (still cross-compile-
-- unverified) JNI bridge will eventually call from Android: materialize,
-- render, enumerate legal actions, fire one, repeat. Unlike the bridge's
-- own single-shot functions (one call in, one call out, no history),
-- this REPL accumulates every fired transition as its own real,
-- independently-provenanced 'IdentifiedCommit' -- same accumulation
-- discipline 'examples/cascade-demo/run.sh' already proved out
-- (fire, apply the resulting commit into the running world, re-check,
-- repeat), just interactive instead of a fixed hand-specified script.
--
-- Usage: interactive-browser <world.dmml> <machine.dmml>
module Main (main) where

import qualified Data.ByteString as BS
import Data.List (isPrefixOf)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.IO (hFlush, stdout)
import Text.Megaparsec (errorBundlePretty)
import Text.Read (readMaybe)

import DMML.Ast (MachineStmt, NodeRef (..), machineNode, nodeRefSegments)
import DMML.Fire (FireError, fireTransition, renderFiredCommit)
import DMML.Guard (EvalContext (..), availableTransitions)
import DMML.LocalIdentity (localFileRef)
import DMML.Materialize (IdentifiedCommit (..), WorldSnapshot, applyIdentifiedCommits, renderSnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

main :: IO ()
main = do
  args <- getArgs
  case args of
    [worldPath, machinePath] -> do
      worldRaw <- BS.readFile worldPath
      worldStmt <- case parseCommitSurface (TE.decodeUtf8 worldRaw) of
        Right stmt -> pure stmt
        Left err -> putStrLn (worldPath <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"
      machine <- do
        src <- TIO.readFile machinePath
        case parseMachineSurface src of
          Right m -> pure m
          Left err -> putStrLn (machinePath <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"
      let initial = IdentifiedCommit {icRef = localFileRef worldPath worldRaw, icCommit = worldStmt}
          selfNode = "player/one" :: Text
      putStrLn "-- interactive-browser: type an action's number to fire it, 'q' to quit --\n"
      loop [initial] machine selfNode (1 :: Int)
    _ -> putStrLn "usage: interactive-browser <world.dmml> <machine.dmml>" >> exitFailure

loop :: [IdentifiedCommit] -> MachineStmt -> Text -> Int -> IO ()
loop history machine selfNode round_ = do
  let snap = applyIdentifiedCommits "world" history
      machineKey = nodeRefText (machineNode machine)
      machineMap = Map.singleton machineKey machine
      ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
      actions = availableTransitions machineMap ctx snap

  putStrLn (replicate 60 '=')
  putStrLn ("round " <> show round_)
  putStrLn (replicate 60 '=')
  TIO.putStr (renderSnapshot snap)
  putStrLn ""

  if null actions
    then putStrLn "Nothing more you can do here."
    else do
      putStrLn "You can:"
      mapM_ (\(i, (m, t)) -> putStrLn ("  [" <> show i <> "] " <> T.unpack m <> "/" <> T.unpack t)) (zip [1 :: Int ..] actions)
      putStr "\n> "
      hFlush stdout
      line <- getLine
      if "q" `isPrefixOf` line
        then putStrLn "bye."
        else case readMaybe line >>= \i -> lookup i (zip [1 ..] actions) of
          Nothing -> putStrLn "not a valid choice.\n" >> loop history machine selfNode round_
          Just (_, transitionIdent) ->
            case fireTransition machineMap machine transitionIdent ctx snap of
              Left err -> do
                putStrLn ("refused: " <> describeError err <> "\n")
                loop history machine selfNode round_
              Right effects -> do
                let commitLabel = "fire" <> T.pack (show round_)
                    commitText = renderFiredCommit commitLabel effects
                case parseCommitSurface commitText of
                  Left _ -> do
                    -- A legal transition with no real effects (e.g. a
                    -- bare guard-only transition) renders to nothing
                    -- parseable -- nothing new to add to history, but
                    -- not an error either.
                    putStrLn "(that action had no visible effect)\n"
                    loop history machine selfNode (round_ + 1)
                  Right stmt -> do
                    let newRef = localFileRef (T.unpack commitLabel) (TE.encodeUtf8 commitText)
                        newCommit = IdentifiedCommit {icRef = newRef, icCommit = stmt}
                    putStrLn "-- fired --"
                    TIO.putStr commitText
                    putStrLn ""
                    loop (history ++ [newCommit]) machine selfNode (round_ + 1)

describeError :: FireError -> String
describeError = show
