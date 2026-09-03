{-# LANGUAGE OverloadedStrings #-}

-- | CLI: fires one named transition on a real machine against a real
-- world snapshot, and prints the resulting DMML Surface commit text --
-- Phase 3 of the 2026-09-03 authoring-tools plan ("machines should
-- govern all transitions... build phase 2/3 -- they're the same").
-- Prints nothing else on success: the point is that the printed commit
-- is real, re-parseable DMML, pipeable straight into @validate-commit@\/
-- @check-declared@\/@retro-gate@ the same way any hand-authored commit
-- is, not a description of what would happen.
--
-- Usage: fire-transition <machine.dmml> <transition> <verb>
--          [--world <file.dmml>]... [--param <name>=<value>]...
module Main (main) where

import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (NodeRef (..), machineNode)
import DMML.Fire (FireError (..), fireTransition, renderFiredCommit)
import DMML.Guard (EvalContext (..))
import DMML.Materialize (applyCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

data Args = Args
  { argMachineFile :: FilePath
  , argTransition :: T.Text
  , argVerb :: T.Text
  , argWorldFiles :: [FilePath]
  , argParams :: [(T.Text, T.Text)]
  }

usage :: String
usage =
  "usage: fire-transition <machine.dmml> <transition> <verb>\n"
    ++ "         [--world <file.dmml>]... [--param <name>=<value>]..."

parseArgs :: [String] -> Either String Args
parseArgs (machineFile : transition : verb : rest) = go rest [] []
  where
    go [] worlds params =
      Right
        Args
          { argMachineFile = machineFile
          , argTransition = T.pack transition
          , argVerb = T.pack verb
          , argWorldFiles = reverse worlds
          , argParams = reverse params
          }
    go ("--world" : f : more) worlds params = go more (f : worlds) params
    go ("--param" : kv : more) worlds params = case break (== '=') kv of
      (k, '=' : v) -> go more worlds ((T.pack k, T.pack v) : params)
      _ -> Left ("--param expects name=value, got " <> kv)
    go (other : _) _ _ = Left ("unrecognized argument " <> other)
parseArgs _ = Left usage

main :: IO ()
main = do
  rawArgs <- getArgs
  case parseArgs rawArgs of
    Left err -> putStrLn err >> exitFailure
    Right args -> run args

run :: Args -> IO ()
run args = do
  machineSrc <- TIO.readFile (argMachineFile args)
  case parseMachineSurface machineSrc of
    Left err -> putStrLn (argMachineFile args <> ":\n" <> errorBundlePretty err) >> exitFailure
    Right machine -> do
      worldSrcs <- mapM TIO.readFile (argWorldFiles args)
      commits <- mapM parseWorldFile (zip (argWorldFiles args) worldSrcs)
      let snap = applyCommits "world" commits
          selfNode = T.intercalate "/" (nodeRefSegments (machineNode machine))
          ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.fromList (argParams args)}
      case fireTransition machine (argTransition args) ctx snap of
        Left err -> putStrLn ("fire-transition: refused -- " <> describeError err) >> exitFailure
        Right facts -> TIO.putStr (renderFiredCommit (argVerb args) facts)
  where
    parseWorldFile (path, src) = case parseCommitSurface src of
      Right c -> pure c
      Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure

describeError :: FireError -> String
describeError FireNotDeclared = "no such transition declared on this machine"
describeError FireBlocked = "transition's guards do not currently hold"
describeError (FireUnresolvedSubject eff) = "an effect's subject term did not resolve: " <> show eff
describeError (FireUnresolvedValue eff) = "an effect's asserted value term did not resolve: " <> show eff
describeError (FireRetractNeedsProvenance eff) =
  "a retract effect needs real commit provenance (uri#cid) this CLI cannot synthesize from a snapshot alone: "
    <> show eff
