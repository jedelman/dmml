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
-- @{id, status, output?, var?, candidates?, reads?}@ where status is one of
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
import Data.List (foldl', nub)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast
  ( ExistsExpr (..)
  , GuardClause (..)
  , MachineStmt
  , Pattern (..)
  , PatternHop (..)
  , PatternTerm (..)
  , machineNode
  , nodeRefSegments
  )
import DMML.Fire (FireError (..), ResolvedEffect (..), fireTransition, renderFiredCommits, renderFiredMachine)
import DMML.Guard (EvalContext (..), GuardError (..), lookupTransition, resolveTransition)
import DMML.LocalIdentity (localFileRef)
import DMML.Materialize (IdentifiedCommit (..), WorldSnapshot, applyIdentifiedCommit, applyIdentifiedCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

data Cand = Cand
  { cId :: Text
  , cMachine :: FilePath
  , cTransition :: Text
  , cVerb :: Text
  , cParams :: [(Text, Text)]
  , -- | Extra commit files applied ON TOP of the shared snapshot, for
    -- this candidate only.
    --
    -- Added 2026-09-19 for the conflict probe in the Jev driver's
    -- `group_into_generations`, which asks "if A fires, is B still
    -- legal and unchanged?" -- one probe per PAIR, each needing A's
    -- output layered over the same base world. It was doing that with a
    -- `fire-transition` subprocess per pair, each re-materializing
    -- everything: measured at 6.21s to group twelve candidates, against
    -- 0.15s to scan all 144. Forty times the cost of the entire round's
    -- scan, in the step that merely checks the scan's results for
    -- conflicts, and quadratic in candidates.
    --
    -- That cost was disclosed in the driver's own docstring when it was
    -- written ("a real, disclosed cost to revisit if the cannon ever
    -- produces enough candidates in one round to make that quadratic
    -- cost matter"). It now does.
    --
    -- The base world is still materialized ONCE for the whole call; an
    -- extra file is parsed once per distinct path and applied
    -- incrementally per candidate, so N-squared probes cost one process
    -- and N-squared cheap snapshot extensions.
    cExtra :: [FilePath]
  }

instance A.FromJSON Cand where
  parseJSON = A.withObject "candidate" $ \o ->
    Cand
      <$> o A..: "id"
      <*> o A..: "machine"
      <*> o A..: "transition"
      <*> o A..:? "verb" A..!= "acts"
      <*> (AKM.toList <$> o A..:? "params" A..!= mempty >>= traverse pair)
      <*> o A..:? "extra_world" A..!= []
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
      -- Each distinct extra file is parsed once, however many candidates
      -- layer it. The driver's conflict probe reuses one probe file
      -- across every pair sharing the same A, so this matters.
      let extraPaths = nub (concatMap cExtra cands)
      extras <- Map.fromList . zip extraPaths <$> mapM parseWorldFile extraPaths
      let machines = Map.fromList [(nodeRefText m, m) | m <- loaded]
          byPath = Map.fromList (zip (nub (machineFiles ++ map cMachine cands)) loaded)
          snapFor c = case [e | path <- cExtra c, Just e <- [Map.lookup path extras]] of
            [] -> snap
            es -> foldl' (flip (applyIdentifiedCommit "probe")) snap es
      BL.putStrLn (A.encode [scan (snapFor c) machines byPath c | c <- cands])
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

-- | Every @(subject, predicate)@ slot this candidate's GUARDS read,
-- with 'Nothing' for "any subject".
--
-- Reported so a caller can decide whether two firings can interact
-- WITHOUT simulating the pair. The Jev driver's
-- @group_into_generations@ needs exactly this: firing A can only change
-- B's verdict if something A writes or spends is something B reads, and
-- what B reads is its guards, which appear nowhere in its output.
--
-- It was deriving that with a regex over the machine file -- a second,
-- weaker parser for a language that already has one, right here. Two
-- things the real AST gets that the regex could not:
--
-- * the IMPLICIT @(self, state, from)@ guard that 'resolveTransition'
--   prepends for a @from -> to@ transition. It is not in the file's
--   guard lines at all, so a text reader cannot see it.
-- * a @$param@ or a pre-bound @?binder@ resolved to its ACTUAL subject
--   rather than widened to a wildcard, since the candidate carries its
--   own params. Narrower, and narrower for the right reason: this
--   mirrors 'DMML.Guard.resolveTerm' case for case rather than guessing
--   at its behaviour.
--
-- Conservative wherever the subject genuinely is not knowable before
-- the guard runs: an unbound anchor makes 'DMML.Guard.evalExists' start
-- from every subject in the world, and a binder's witness is by
-- definition not known until the walk finds it. Both report 'Nothing',
-- which a caller must read as "any subject with this predicate".
--
-- Only the SUBJECT of each hop is a read. A hop's object term
-- constrains which walks survive ('DMML.Guard.stepHop' filters on it);
-- it is not itself a separate slot the guard looks up.
guardReads :: Text -> Map.Map Text Text -> MachineStmt -> Text -> [(Maybe Text, Text)]
guardReads selfNode params machine tname =
  case lookupTransition machine tname of
    Nothing -> []
    Just decl ->
      nub
        [ slot
        | g <- fst (resolveTransition decl)
        , slot <- patternReads (existsPattern (guardExists g))
        ]
  where
    patternReads pat = walk (staticSubject (patternAnchor pat)) (patternHops pat)
    walk _ [] = []
    walk subj (hop : more) = (subj, hopPredicate hop) : walk (staticSubject (hopTerm hop)) more

    -- The static counterpart of 'DMML.Guard.resolveTerm', case for
    -- case. 'ctxBindings' is necessarily empty here -- a binder's
    -- witness is what the walk is looking for -- so 'TermBind' falls
    -- through to the param lookup exactly as resolveTerm's own fallback
    -- does, and is a wildcard only when nothing pre-bound it.
    staticSubject TermSelf = Just selfNode
    staticSubject (TermNode n) = Just n
    staticSubject (TermParam name) = Map.lookup name params
    staticSubject (TermBind v) = Map.lookup v params
    staticSubject (TermVar _) = Nothing

readsJson :: [(Maybe Text, Text)] -> A.Value
readsJson = A.toJSON . map one
  where
    one (subj, pred') = obj [("subject", maybe A.Null A.toJSON subj), ("predicate", A.toJSON pred')]

-- | One candidate against the shared snapshot. Identical evaluation to
-- what @fire-transition@ does for a single call -- same 'fireTransition',
-- same context shape -- only the snapshot is reused.
scan :: WorldSnapshot -> Map.Map Text MachineStmt -> Map.Map FilePath MachineStmt -> Cand -> A.Value
scan snap machines byPath c =
  case Map.lookup (cMachine c) byPath of
    Nothing -> obj [("id", A.toJSON (cId c)), ("status", "error"), ("error", "machine file not loaded")]
    Just machine ->
      let params = Map.fromList (cParams c)
          ctx =
            EvalContext
              { ctxSelfNode = nodeRefText machine
              , ctxParams = params
              , ctxBindings = Map.empty
              }
          reads' = ("reads", readsJson (guardReads (nodeRefText machine) params machine (cTransition c)))
       in case fireTransition machines machine (cTransition c) ctx snap of
            Right effects ->
              obj
                [ ("id", A.toJSON (cId c))
                , ("status", "legal")
                , ("output", A.toJSON (render effects))
                , reads'
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
                , reads'
                ]
            Left FireBlocked -> obj [("id", A.toJSON (cId c)), ("status", "blocked"), reads']
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
