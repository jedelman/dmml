{-# LANGUAGE DeriveGeneric #-}
{-# LANGUAGE OverloadedStrings #-}

-- | A resumable "guardian" process: watches a @commits/@ directory,
-- tracks per-(subject, predicate) Shannon entropy over a sliding window
-- of the real, undeduped assertions it sees, and when it detects a
-- rapid local entropy drop that ISN'T explained by a real governed
-- resolution ('DMML.Governance.arbitrate' returning 'Resolved'), mints
-- a real @EntropyCollapse@ DMML commit reporting the finding -- parse-
-- verified before being written, same discipline the old (now removed)
-- Contest-minting always used.
--
-- Design, from the 2026-09-02 conversation this implements:
--
--   * Entropy is climbing forever on an ungoverned pair is FINE, not a
--     problem -- "let them fight." Only a fast, LOCAL drop within a
--     sliding window is even a candidate signal (see 'DMML.Entropy').
--   * A candidate drop explained by real governance (a legitimate
--     transition actually firing) is not collapse, it's the system
--     working -- 'arbitrate' is checked before anything gets minted.
--   * The checkpoint (how far this process has gotten) is ordinary
--     process bookkeeping, not world content -- it lives in a plain
--     JSON file, never as a DMML commit. Only the DETECTION FINDING
--     itself becomes real, sovereign content -- the same "a Drift node,
--     not a narrated guess" precedent this project already established
--     for cross-repo materialization drift.
--   * This process needs nothing beyond what already exists on disk:
--     multiplicity, timestamps (via file ordering), and "was this
--     resolved" are all already native to the substrate (every
--     assertion is its own real commit) -- this is a pure read-and-
--     derive layer over that, not a change to the core data model.
--
-- Usage:
--   entropy-sidecar <commits-dir> <checkpoint-file> [window] [threshold] [--watch SECONDS]
--     window:    how many recent samples of a pair's entropy to keep
--                (default 5)
--     threshold: how negative a window's entropy delta must be to be a
--                CANDIDATE collapse, before attribution is checked
--                (default 1.0 bit)
--     --watch N: after processing everything currently on disk, sleep N
--                seconds and check for new files, forever. Omit for a
--                one-shot batch pass (e.g. backfilling against an
--                already-complete corpus, or a test).
module Main (main) where

import Control.Concurrent (threadDelay)
import Control.Monad (foldM)
import Data.Aeson (FromJSON, ToJSON, decode, encode)
import qualified Data.ByteString.Lazy as BL
import Data.List (isSuffixOf, sort)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import GHC.Generics (Generic)
import System.Directory (doesFileExist, listDirectory)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import Text.Megaparsec (errorBundlePretty)
import Text.Printf (printf)

import DMML.Ast
import DMML.Entropy
import DMML.Governance (GovernedOutcome (..), arbitrate)
import DMML.Materialize (WorldSnapshot, applyCommit, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

predText :: PredicateRef -> Text
predText RdfType = "a"
predText (PredIdent t) = t

valueText :: Value -> Text
valueText (ValueNode n) = nodeRefText n
valueText (ValueLiteral (LitString s)) = s
valueText (ValueLiteral (LitNumber n)) = n
valueText (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

-- Checkpoint: plain process bookkeeping, not world content -- see this
-- module's own doc comment.
--
-- MUST persist raw value-counts, not just the entropy samples derived
-- from them -- a real bug, caught by actually killing and resuming this
-- process against a real scenario, not assumed correct: entropy at
-- resume time depends on the FULL cumulative count of every value ever
-- seen, and checkpointing only the entropy history meant a resumed run
-- silently recomputed counts from zero at the resume point, producing
-- entropy values that didn't match what an uninterrupted run would have
-- shown (deltas roughly 4x too large in the actual reproduction, from
-- the artificially shrunk denominator).
data CheckpointPair = CheckpointPair
  { cpSubject :: Text
  , cpPredicate :: Text
  , cpHistory :: [(Int, Double)]
  , cpCounts :: [(Text, Int)]
  }
  deriving (Generic, Show)

instance ToJSON CheckpointPair
instance FromJSON CheckpointPair

data Checkpoint = Checkpoint
  { ckProcessedCount :: Int
  , ckPairs :: [CheckpointPair]
  , ckAlertCount :: Int
  }
  deriving (Generic, Show)

instance ToJSON Checkpoint
instance FromJSON Checkpoint

freshCheckpoint :: Checkpoint
freshCheckpoint = Checkpoint 0 [] 0

loadCheckpoint :: FilePath -> IO Checkpoint
loadCheckpoint path = do
  exists <- doesFileExist path
  if not exists
    then pure freshCheckpoint
    else do
      raw <- BL.readFile path
      pure (maybe freshCheckpoint id (decode raw))

saveCheckpoint :: FilePath -> Checkpoint -> IO ()
saveCheckpoint path ck = BL.writeFile path (encode ck)

historiesFromCheckpoint :: Checkpoint -> Map (Text, Text) WindowedHistory
historiesFromCheckpoint ck =
  Map.fromList
    [ ((cpSubject p, cpPredicate p), [RoundSample i h | (i, h) <- cpHistory p])
    | p <- ckPairs ck
    ]

-- | Restores exact raw value-counts, not just their derived entropy --
-- see this file's Checkpoint doc comment for why approximating this
-- from the entropy history alone is wrong.
rawCountsFromCheckpoint :: Checkpoint -> Map (Text, Text) (Map Text Int)
rawCountsFromCheckpoint ck =
  Map.fromList
    [ ((cpSubject p, cpPredicate p), Map.fromList (cpCounts p))
    | p <- ckPairs ck
    ]

checkpointFromState :: Int -> Int -> Map (Text, Text) WindowedHistory -> Map (Text, Text) (Map Text Int) -> Checkpoint
checkpointFromState processed alerts histories rawCounts =
  Checkpoint
    { ckProcessedCount = processed
    , ckAlertCount = alerts
    , ckPairs =
        [ CheckpointPair subj pred_ [(sampleIndex s, sampleEntropy s) | s <- hist] (Map.toList (Map.findWithDefault Map.empty (subj, pred_) rawCounts))
        | ((subj, pred_), hist) <- Map.toList histories
        ]
    }

-- | Loop state threaded across processed files: the running snapshot
-- (for arbitrate), the machine map (ditto), raw undeduped per-pair
-- value counts (for entropy -- deliberately separate from
-- WorldSnapshot's own deduped Alternatives), and each pair's windowed
-- entropy history.
data LoopState = LoopState
  { lsSnapshot :: WorldSnapshot
  , lsMachines :: Map Text MachineStmt
  , lsRawCounts :: Map (Text, Text) (Map Text Int)
  , lsHistories :: Map (Text, Text) WindowedHistory
  , lsAlertCount :: Int
  }

classify :: Text -> Either String (Either CommitStmt MachineStmt)
classify src = case parseCommitSurface src of
  Right stmt -> Right (Left stmt)
  Left commitErr -> case parseMachineSurface src of
    Right m -> Right (Right m)
    Left _ -> Left (errorBundlePretty commitErr)

nextSeq :: FilePath -> IO Int
nextSeq dir = do
  files <- listDirectory dir
  let nums = [n | f <- files, ".dmml" `isSuffixOf` f, let digits = takeWhile (`elem` ("0123456789" :: String)) f, not (null digits), let n = read digits :: Int]
  pure (if null nums then 1 else maximum nums + 1)

mintAlert :: Int -> (Text, Text) -> Double -> Double -> Int -> Text
mintAlert n (subj, pred_) before after windowFiles =
  T.unlines
    [ "commit flags"
    , "  declare relation subject"
    , "  declare relation predicate"
    , "  declare attribute entropyBefore"
    , "  declare attribute entropyAfter"
    , "  declare attribute windowFiles"
    , ""
    , "  " <> node <> " :: a EntropyCollapse"
    , "  " <> node <> " . subject = " <> subj
    , "  " <> node <> " . predicate = " <> pred_
    , "  " <> node <> " . entropyBefore = " <> fmtD before
    , "  " <> node <> " . entropyAfter = " <> fmtD after
    , "  " <> node <> " . windowFiles = " <> T.pack (show windowFiles)
    , "  " <> node <> " `disputes` " <> subj
    ]
  where
    -- Node ref segments can't contain hyphens -- confirmed the hard way,
    -- the same class of mistake this project's own dmml-hs endurance
    -- work already hit once before (agent-authored content needing "no
    -- hyphens in identifiers" spelled out explicitly).
    node = "entropy_alert/" <> slug subj <> "_" <> pred_ <> "_" <> T.pack (show n)
    slug = T.map (\c -> if c == '/' then '_' else c)
    fmtD d = T.pack (printf "%.4f" (d :: Double))

-- | Processes one already-classified file, updating and returning the
-- new loop state. Mints and writes a real alert file as a side effect
-- when a candidate collapse turns out unattributed -- the only IO this
-- function does beyond that is printing what it found.
processFile :: Double -> Int -> FilePath -> FilePath -> Int -> Either CommitStmt MachineStmt -> LoopState -> IO LoopState
processFile threshold window commitsDir _path fileIdx item st = case item of
  Right machine ->
    pure st {lsMachines = Map.insert (nodeRefText (machineNode machine)) machine (lsMachines st)}
  Left commit -> do
    -- Exclude this sidecar's OWN minted alert facts from being tracked
    -- as monitored pairs -- a real bug, caught the hard way running a
    -- real resume test: alert files sit in the same commits/ directory
    -- as everything else and get reprocessed as ordinary content on a
    -- later pass, and without this filter the sidecar starts tracking
    -- "entropy over its own bookkeeping fields" as if they were
    -- disputed game facts. Doesn't cause false collapse alerts about
    -- the REAL disputed pair (an alert commit never touches that pair's
    -- own predicate), but it's wrong and it bloats the checkpoint
    -- unboundedly as more alerts get minted.
    let touched =
          [ (nodeRefText (factSubject f), predText (factPredicate f), factValue f)
          | ItemFact f <- commitItems commit
          , not ("entropy_alert/" `T.isPrefixOf` nodeRefText (factSubject f))
          ]
        snapshot' = applyCommit "corpus" (lsSnapshot st) commit
        rawCounts' =
          foldr
            (\(subj, pred_, v) m -> Map.insertWith (Map.unionWith (+)) (subj, pred_) (Map.singleton (valueText v) 1) m)
            (lsRawCounts st)
            touched
        pairsTouched = [(s, p) | (s, p, _) <- touched]
    (histories', alertCount') <-
      foldM
        (\(hs, alerts) key -> do
          let counts = Map.findWithDefault Map.empty key rawCounts'
              h = shannonEntropy counts
              hist' = recordSample window fileIdx h (Map.findWithDefault [] key hs)
              hs' = Map.insert key hist' hs
          case windowDelta hist' of
            Just delta | delta <= negate threshold -> do
              let outcome = arbitrate (lsMachines st) key snapshot'
              case outcome of
                Resolved _ _ -> do
                  putStrLn ("[entropy] " <> show key <> ": entropy dropped " <> show delta <> " bits, but a real resolution explains it -- not collapse")
                  pure (hs', alerts)
                _ -> do
                  let n = alerts + 1
                      before = maybe h sampleEntropy (safeLast hist')
                      alertText = mintAlert n key before h window
                  case parseCommitSurface alertText of
                    Left err -> do
                      putStrLn ("[entropy] BUG: minted alert failed to parse: " <> errorBundlePretty err)
                      pure (hs', alerts)
                    Right _ -> do
                      seqN <- nextSeq commitsDir
                      let outPath = commitsDir </> (printf "%04d-entropy-collapse-%d.dmml" seqN n)
                      TIO.writeFile outPath alertText
                      putStrLn ("[entropy] COLLAPSE detected, unattributed: " <> show key <> " -- entropy fell " <> show delta <> " bits (no governed resolution explains it). Wrote " <> outPath)
                      pure (hs', n)
            _ -> pure (hs', alerts)
        )
        (lsHistories st, lsAlertCount st)
        pairsTouched
    pure
      st
        { lsSnapshot = snapshot'
        , lsRawCounts = rawCounts'
        , lsHistories = histories'
        , lsAlertCount = alertCount'
        }
  where
    safeLast [] = Nothing
    safeLast xs = Just (last xs)

main :: IO ()
main = do
  args <- getArgs
  case args of
    (commitsDir : checkpointPath : rest) -> do
      let (posArgs, watchArgs) = break (== "--watch") rest
          window = case posArgs of (w : _) -> read w; _ -> 5
          threshold = case posArgs of (_ : t : _) -> read t; _ -> 1.0
          watchSeconds = case watchArgs of ["--watch", n] -> Just (read n :: Int); _ -> Nothing
      ck <- loadCheckpoint checkpointPath
      runLoop commitsDir checkpointPath window threshold watchSeconds ck
    _ ->
      putStrLn "usage: entropy-sidecar <commits-dir> <checkpoint-file> [window] [threshold] [--watch SECONDS]"
        >> exitFailure

runLoop :: FilePath -> FilePath -> Int -> Double -> Maybe Int -> Checkpoint -> IO ()
runLoop commitsDir checkpointPath window threshold watchSeconds ck0 = go ck0
  where
    go ck = do
      allFiles <- sort . filter (".dmml" `isSuffixOf`) <$> listDirectory commitsDir
      let newFiles = drop (ckProcessedCount ck) allFiles
      if null newFiles
        then case watchSeconds of
          Nothing -> putStrLn ("[entropy] done -- " <> show (ckProcessedCount ck) <> " file(s) processed, " <> show (ckAlertCount ck) <> " alert(s) minted")
          Just secs -> threadDelay (secs * 1000000) >> go ck
        else do
          let st0 =
                LoopState
                  { lsSnapshot = emptySnapshot
                  , lsMachines = Map.empty
                  , lsRawCounts = rawCountsFromCheckpoint ck
                  , lsHistories = historiesFromCheckpoint ck
                  , lsAlertCount = ckAlertCount ck
                  }
          finalSt <-
            foldM
              ( \(idx, st) f -> do
                  src <- TIO.readFile (commitsDir </> f)
                  case classify src of
                    Left err -> do
                      putStrLn ("[entropy] skipping unparseable " <> f <> ": " <> err)
                      pure (idx + 1, st)
                    Right item -> do
                      st' <- processFile threshold window commitsDir f idx item st
                      -- idx+1, not idx: this file (at position idx) has
                      -- now been processed -- a real off-by-one, caught
                      -- by an actual resume test showing 9 processed
                      -- after 10 files went in.
                      let ck' = checkpointFromState (idx + 1) (lsAlertCount st') (lsHistories st') (lsRawCounts st')
                      saveCheckpoint checkpointPath ck'
                      pure (idx + 1, st')
              )
              (ckProcessedCount ck, st0)
              newFiles
              >>= \(finalIdx, st) -> pure (checkpointFromState finalIdx (lsAlertCount st) (lsHistories st) (lsRawCounts st))
          case watchSeconds of
            Nothing -> putStrLn ("[entropy] done -- " <> show (ckProcessedCount finalSt) <> " file(s) processed, " <> show (ckAlertCount finalSt) <> " alert(s) minted")
            Just secs -> threadDelay (secs * 1000000) >> go finalSt
