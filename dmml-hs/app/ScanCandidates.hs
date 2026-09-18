{-# LANGUAGE OverloadedStrings #-}

-- | CLI: dry-fire MANY candidates against ONE materialized world.
--
-- Exists for a measured reason. @app\/FireTransition.hs@ parses every
-- @--world@ file and rebuilds the whole snapshot on every invocation,
-- and the Jev driver calls it once per candidate per round -- so a round
-- costs (candidates x world-files) parses of the same unchanged files.
-- Measured 2026-09-18: one dry-fire against 3200 facts takes 0.39s, so
-- a 200-candidate round at that size costs ~78 seconds, essentially all
-- of it re-reading files that did not change. The engine itself is
-- linear and cheap (~0.12ms per fact); the cost was entirely the
-- repetition.
--
-- This binary materializes once and reuses the snapshot for every
-- candidate, collapsing a round from N materializations to one. Nothing
-- about evaluation changes -- each candidate goes through the identical,
-- unmodified 'DMML.Fire.fireTransition' with its own 'EvalContext'.
--
-- Deliberately a separate binary rather than a mode on fire-transition:
-- that CLI's contract is "fire one transition, print the commit," which
-- is what a person runs and what cascade-demo\/run.sh depends on.
-- Batch scanning has a different contract (JSON in, JSON out, no
-- firing), and folding two contracts into one entry point would have
-- made both worse.
--
-- Usage:
--   scan-candidates <candidates.json> [--world f]... [--machine f]...
--
-- Input is a JSON array of
-- @{id, machine, transition, verb, params}@; output a JSON array of
-- @{id, status, output?, var?, candidates?}@ where status is one of
-- @legal@, @blocked@, @ambiguous@, @error@. An @ambiguous@ entry
-- carries the binder name and every witness, exactly as
-- 'DMML.Guard.GuardAmbiguousBinding' reports them -- that refusal is a
-- question for whoever is choosing, and this is how it reaches them
-- without anyone parsing prose.
module Main (main) where

import qualified Data.Aeson as A
import qualified Data.Aeson.Key as AK
import qualified Data.Aeson.KeyMap as AKM
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy.Char8 as BL
import Data.List (nub)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt, machineNode, nodeRefSegments)
import DMML.Fire (FireError (..), ResolvedEffect (..), fireTransition, renderFiredCommits, renderFiredMachine)
import DMML.Guard (EvalContext (..), GuardError (..))
import DMML.LocalIdentity (localFileRef)
import DMML.Materialize (IdentifiedCommit (..), WorldSnapshot, applyIdentifiedCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

data Cand = Cand
  { cId :: Text
  , cMachine :: FilePath
  , cTransition :: Text
  , cVerb :: Text
  , cParams :: [(Text, Text)]
  }

instance A.FromJSON Cand where
  parseJSON = A.withObject "candidate" $ \o ->
    Cand
      <$> o A..: "id"
      <*> o A..: "machine"
      <*> o A..: "transition"
      <*> o A..:? "verb" A..!= "acts"
      <*> (AKM.toList <$> o A..:? "params" A..!= mempty >>= traverse pair)
    where
      pair (k, v) = (,) (AK.toText k) <$> A.parseJSON v

nodeRefText :: MachineStmt -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments . machineNode

obj :: [(Text, A.Value)] -> A.Value
obj = A.object . map (\(k, v) -> AK.fromText k A..= v)

main :: IO ()
main = do
  args <- getArgs
  case args of
    (candPath : rest) -> do
      let worlds = flagged "--world" rest
          machineFiles = flagged "--machine" rest
      raw <- BL.readFile candPath
      cands <- case A.eitherDecode raw of
        Left e -> putStrLn ("scan-candidates: bad candidates JSON: " <> e) >> exitFailure
        Right cs -> pure cs
      identified <- mapM parseWorldFile worlds
      -- Materialize ONCE. This is the whole point of the binary.
      let snap = applyIdentifiedCommits "world" identified
      loaded <- mapM loadMachine (nub (machineFiles ++ map cMachine cands))
      let machines = Map.fromList [(nodeRefText m, m) | m <- loaded]
          byPath = Map.fromList (zip (nub (machineFiles ++ map cMachine cands)) loaded)
      BL.putStrLn (A.encode [scan snap machines byPath c | c <- cands])
    _ -> putStrLn "usage: scan-candidates <candidates.json> [--world f]... [--machine f]..." >> exitFailure
  where
    flagged flag = go
      where
        go (f : v : more) | f == flag = v : go more
        go (_ : more) = go more
        go [] = []

    parseWorldFile path = do
      raw <- BS.readFile path
      case parseCommitSurface (TE.decodeUtf8 raw) of
        Right c -> pure IdentifiedCommit {icRef = localFileRef path raw, icCommit = c}
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure

    loadMachine path = do
      src <- TIO.readFile path
      case parseMachineSurface src of
        Right m -> pure m
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure

-- | One candidate against the shared snapshot. Identical evaluation to
-- what @fire-transition@ does for a single call -- same 'fireTransition',
-- same context shape -- only the snapshot is reused.
scan :: WorldSnapshot -> Map.Map Text MachineStmt -> Map.Map FilePath MachineStmt -> Cand -> A.Value
scan snap machines byPath c =
  case Map.lookup (cMachine c) byPath of
    Nothing -> obj [("id", A.toJSON (cId c)), ("status", "error"), ("error", "machine file not loaded")]
    Just machine ->
      let ctx =
            EvalContext
              { ctxSelfNode = nodeRefText machine
              , ctxParams = Map.fromList (cParams c)
              , ctxBindings = Map.empty
              }
       in case fireTransition machines machine (cTransition c) ctx snap of
            Right effects ->
              obj
                [ ("id", A.toJSON (cId c))
                , ("status", "legal")
                , ("output", A.toJSON (render effects))
                ]
            -- An ambiguous binder is NOT a failure. It is a decision the
            -- engine refuses to make, carried out whole so the driver can
            -- put it to whoever is choosing.
            Left (FireGuardError (GuardAmbiguousBinding v cands)) ->
              obj
                [ ("id", A.toJSON (cId c))
                , ("status", "ambiguous")
                , ("var", A.toJSON v)
                , ("candidates", A.toJSON cands)
                ]
            Left FireBlocked -> obj [("id", A.toJSON (cId c)), ("status", "blocked")]
            Left err ->
              obj
                [ ("id", A.toJSON (cId c))
                , ("status", "error")
                , ("error", A.toJSON (T.pack (show err)))
                ]
  where
    render effects =
      T.concat (map (<> "\n") (renderFiredCommits (cVerb c) effects))
        <> T.concat ["\n" <> renderFiredMachine m | ResolvedSpawn m <- effects]
