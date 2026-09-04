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
-- UPDATED 2026-09-04 (jedelman/dmml#4): every @--world@ file is now
-- materialized WITH real provenance ('DMML.Materialize.
-- applyIdentifiedCommit'), its 'DMML.Ast.StrongRef' computed from the
-- file's own exact bytes ('DMML.LocalIdentity.localFileRef') -- not a
-- real atproto CID (see that module's own doc comment for why), but real
-- enough that a retract effect can cite it honestly and 'DMML.Fire' can
-- build an actual @consumes@ block instead of refusing outright.
--
-- Usage: fire-transition <machine.dmml> <transition> <verb>
--          [--world <file.dmml>]... [--param <name>=<value>]...
module Main (main) where

import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (NodeRef (..), machineNode)
import DMML.Fire (FireError (..), fireTransition, renderFiredCommit)
import DMML.Guard (EvalContext (..))
import DMML.LocalIdentity (localFileRef)
import DMML.Materialize (IdentifiedCommit (..), applyIdentifiedCommits)
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
      identified <- mapM parseWorldFile (argWorldFiles args)
      let snap = applyIdentifiedCommits "world" identified
          selfNode = T.intercalate "/" (nodeRefSegments (machineNode machine))
          ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.fromList (argParams args)}
      case fireTransition machine (argTransition args) ctx snap of
        Left err -> putStrLn ("fire-transition: refused -- " <> describeError err) >> exitFailure
        Right effects -> TIO.putStr (renderFiredCommit (argVerb args) effects)
  where
    parseWorldFile :: FilePath -> IO IdentifiedCommit
    parseWorldFile path = do
      raw <- BS.readFile path
      case parseCommitSurface (TE.decodeUtf8 raw) of
        Right c -> pure IdentifiedCommit {icRef = localFileRef path raw, icCommit = c}
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure

describeError :: FireError -> String
describeError FireNotDeclared = "no such transition declared on this machine"
describeError FireBlocked = "transition's guards do not currently hold"
describeError (FireUnresolvedSubject eff) = "an effect's subject term did not resolve: " <> show eff
describeError (FireUnresolvedValue eff) = "an effect's asserted value term did not resolve: " <> show eff
describeError (FireRetractNoSuchFact eff) =
  "a retract effect's (subject, predicate) has no live fact to retract: " <> show eff
describeError (FireRetractNoProvenance eff) =
  "a retract effect's (subject, predicate) has a live fact, but it carries no real provenance to cite"
    <> " (materialized without a real StrongRef -- pass it via --world so it gets one): "
    <> show eff
describeError (FireRetractAmbiguous eff) =
  "a retract effect's (subject, predicate) currently has more than one live alternative -- refusing"
    <> " rather than cite just one of several: "
    <> show eff
