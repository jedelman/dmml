{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE DeriveGeneric #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE StandaloneDeriving #-}

-- | Wire format for a checkpoint of a 'WorldSnapshot''s RAW (pre-
-- governance) materialized facts, per commit -- see
-- @app/CheckpointRebuild.hs@ and @written-world@'s
-- @dev-journal/2026-09-02-checkpoint-per-commit.md@ for the mechanism
-- this exists to support: check the checkpoint in as real, git-tracked
-- content, keyed by the @commits/@ tree hash it summarizes, so the next
-- merge only has to fold in the handful of NEW files it introduces
-- against the parent checkpoint, never replay full history.
--
-- Deliberately stores facts as-is (every live 'Alternatives' value),
-- never the post-'DMML.Governance.applyGovernance' collapsed view --
-- 'DMML.Materialize.collapseToOne' is destructive (discards every
-- alternative but the winner), and a commit added after this checkpoint
-- can still introduce a governing machine for a pair that's been
-- sitting multi-valued since before it existed. Governance always gets
-- reapplied fresh, over checkpoint-facts unioned with whatever's new,
-- at READ time -- cheap (a 'Data.Map.Strict.foldl'' over pairs), not a
-- re-parse.
--
-- FIXED 2026-09-04 (jedelman/dmml#7): every alternative's real
-- provenance (a @uri#cid@ citation, if it had one) now round-trips
-- through a checkpoint instead of being silently reset to 'Nothing'.
-- Before this fix, ANY player who rehydrated local state from a
-- checkpoint (rather than full-replaying every commit from genesis)
-- permanently lost the ability to retract a fact that predated that
-- checkpoint -- 'DMML.Fire.fireTransition' genuinely refuses to retract
-- a fact with no real provenance to cite, and rehydration used to
-- manufacture exactly that "no real provenance" state for every single
-- fact, unconditionally. See 'CheckpointFact' below for the wire shape.
--
-- Deliberately does NOT checkpoint machine definitions ('MachineStmt')
-- -- a real, disclosed scope limit, not an oversight. Doing so would
-- need 'GHC.Generics.Generic'\/aeson instances threaded through the
-- whole guard-expression AST ('DMML.Ast.ExistsExpr'\/'Pattern'\/
-- 'PatternHop'\/'PatternTerm'\/'Effect'), a much larger surface than the
-- plain-value types here, for a part of the corpus that stays small in
-- practice (E1's real 200-commit run: a handful of machine files against
-- hundreds of fact commits). Machine defs still get found by scanning
-- @commits/@ each time -- real, un-eliminated cost, but classification-
-- only, not the full materialize-and-dedupe fold this checkpoint exists
-- to skip.
module DMML.Checkpoint
  ( CheckpointFact (..)
  , CheckpointAlternative (..)
  , CheckpointFile (..)
  , snapshotToCheckpoint
  , checkpointToSnapshot
  , encodeCheckpoint
  , decodeCheckpoint
  , foldNewFiles
  , resolveAndFoldCheckpoint
  ) where

import Data.Aeson (FromJSON, ToJSON, decode, encode)
import qualified Data.ByteString.Lazy as BL
import Data.List (isSuffixOf, sortOn)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import GHC.Generics (Generic)
import System.Directory (doesFileExist, listDirectory)
import System.FilePath ((</>))
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (DeclKind (..), Literal (..), NodeRef (..), Span (..), StrongRef (..), Value (..))
import DMML.Materialize (Alternatives (..), WorldSnapshot (..), alternativeEntries, applyCommit, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

-- Standalone deriving: these types are defined in DMML.Ast, which
-- deliberately carries no aeson dependency of its own (see its own doc
-- comment on why -- shared by every module, most of which never need
-- JSON at all). Legal because every constructor and field these types
-- expose is already in scope via the import above.
deriving instance Generic NodeRef

deriving instance Generic Literal

deriving instance Generic Value

deriving instance Generic DeclKind

instance ToJSON NodeRef

instance FromJSON NodeRef

instance ToJSON Literal

instance FromJSON Literal

instance ToJSON Value

instance FromJSON Value

instance ToJSON DeclKind

instance FromJSON DeclKind

-- | One live alternative's own provenance label, value, and (if it had
-- one) real citation -- @(uri, cid)@, not a full 'DMML.Ast.StrongRef'.
-- The third 'StrongRef' field, 'DMML.Ast.Span', is a source-location
-- pointer into the ORIGINAL commit's parse -- nothing meaningful to
-- persist here, and nothing downstream inspects a rehydrated ref's span
-- ('checkpointToSnapshot' fills in an honest placeholder). A record,
-- not a bare tuple, so a future field doesn't need every call site
-- rewritten to match a new tuple arity.
data CheckpointAlternative = CheckpointAlternative
  { caLabel :: Text
  , caCitation :: Maybe (Text, Text)
  , caValue :: Value
  }
  deriving (Generic, Show)

instance ToJSON CheckpointAlternative

instance FromJSON CheckpointAlternative

-- | One (subject, predicate) pair's live alternatives, list-shaped
-- rather than a 'Data.Map.Strict.Map' keyed on a tuple -- aeson has no
-- built-in 'Data.Aeson.ToJSONKey' for an arbitrary tuple, and this
-- avoids needing one. Same idiom 'EntropySidecar.hs'\'s own
-- @CheckpointPair@ already established for exactly this reason.
data CheckpointFact = CheckpointFact
  { cfSubject :: Text
  , cfPredicate :: Text
  , cfAlternatives :: [CheckpointAlternative]
  }
  deriving (Generic, Show)

instance ToJSON CheckpointFact

instance FromJSON CheckpointFact

data CheckpointFile = CheckpointFile
  { ckTreeSha :: Text
  , ckDeclared :: [(Text, DeclKind)]
  , ckFacts :: [CheckpointFact]
  }
  deriving (Generic, Show)

instance ToJSON CheckpointFile

instance FromJSON CheckpointFile

snapshotToCheckpoint :: Text -> WorldSnapshot -> CheckpointFile
snapshotToCheckpoint treeSha snap =
  CheckpointFile
    { ckTreeSha = treeSha
    , ckDeclared = Map.toList (snapshotDeclared snap)
    , ckFacts =
        [ CheckpointFact
            subj
            pred_
            [ CheckpointAlternative label (refCitation ref) v
            | (label, ref, v) <- alternativeEntries alts
            ]
        | ((subj, pred_), alts) <- sortOn fst (Map.toList (snapshotFacts snap))
        ]
    }
  where
    refCitation = fmap (\ref -> (strongRefUri ref, strongRefCid ref))

-- | Rebuilds a real 'DMML.Ast.StrongRef' for any alternative that had a
-- real citation when checkpointed -- see this module's own top-of-file
-- doc comment (jedelman/dmml#7) for why this matters: without it, every
-- rehydrated fact permanently lost retractability. The rebuilt ref's
-- 'DMML.Ast.Span' is a placeholder (@checkpoint:<uri>#<cid>@) -- there
-- is no real source location to recover, and nothing downstream
-- inspects it; only 'strongRefUri'\/'strongRefCid' are ever compared
-- against ('DMML.Fire', citation-integrity checking).
checkpointToSnapshot :: CheckpointFile -> WorldSnapshot
checkpointToSnapshot ck =
  emptySnapshot
    { snapshotDeclared = Map.fromList (ckDeclared ck)
    , snapshotFacts =
        Map.fromList
          [ ( (cfSubject f, cfPredicate f)
            , Alternatives
                [ (caLabel a, rehydrateRef (caCitation a), caValue a)
                | a <- cfAlternatives f
                ]
            )
          | f <- ckFacts ck
          ]
    }
  where
    rehydrateRef Nothing = Nothing
    rehydrateRef (Just (uri, cid)) =
      Just (StrongRef uri cid (Span ("checkpoint:" <> uri <> "#" <> cid)))

encodeCheckpoint :: CheckpointFile -> BL.ByteString
encodeCheckpoint = encode

decodeCheckpoint :: BL.ByteString -> Maybe CheckpointFile
decodeCheckpoint = decode

-- | Fold a list of @.dmml@ files into an existing 'WorldSnapshot',
-- returning the result plus how many were folded as real commits (the
-- rest were machine-definition files, silently skipped -- see below).
-- Extracted from @app/CheckpointRebuild.hs@'s own @foldFiles@ so the
-- checkpoint-parent-lookup/bootstrap-fallback orchestration this
-- exists to support (previously duplicated between
-- @sync-spike/broker/atproto-broker.sh@ and
-- @sync-spike/broker/hooks/post-merge@, now ported to Haskell -- see
-- @written-world@'s @dev-journal/2026-09-07-jgit-canonical-single-
-- implementation.md@) has one real implementation to call, not a third
-- copy of this exact logic.
--
-- A machine-definition file showing up here is the NORMAL case, not a
-- caller error (found the hard way at real endurance scale -- see
-- 'CheckpointRebuild''s own history) -- both the bootstrap fold (seed
-- content always includes machine files alongside real commits) and
-- ordinary steady-state operation (an agent minting a brand-new
-- machine mid-run) can put one in this list. Machines simply don't
-- belong in the raw fact checkpoint at all (this module's own top doc
-- comment) -- silently skipping one here is correct, not information
-- loss.
--
-- A genuine parse failure (neither a commit nor a machine) is real
-- malformed content that shape-validation upstream should already have
-- caught -- this throws (via 'ioError') rather than silently
-- continuing, since a library function calling 'System.Exit.exitFailure'
-- directly would be wrong regardless of how unlikely this path is.
foldNewFiles :: WorldSnapshot -> [FilePath] -> IO (WorldSnapshot, Int)
foldNewFiles = go 0
  where
    go !n snap [] = pure (snap, n)
    go !n snap (path : rest) = do
      src <- TIO.readFile path
      case parseCommitSurface src of
        Right stmt -> go (n + 1) (applyCommit "merge" snap stmt) rest
        Left commitErr -> case parseMachineSurface src of
          Right _ -> go n snap rest
          Left _ ->
            ioError
              ( userError
                  ( path
                      <> ": failed to parse as either a commit or a machine:\n"
                      <> errorBundlePretty commitErr
                  )
              )

-- | The checkpoint parent-lookup\/bootstrap-fallback algorithm itself,
-- ported from what was previously two independent bash copies
-- (@sync-spike\/broker\/atproto-broker.sh@ and @sync-spike\/broker\/
-- hooks\/post-merge@ -- see @written-world@'s @dev-journal\/2026-09-07-
-- jgit-canonical-single-implementation.md@). Deliberately takes the
-- parent\/new tree SHAs as plain already-resolved 'String's rather than
-- calling into 'DMML.Jgit' itself -- this module stays JGit-independent
-- (it's used by tools, like @render-snapshot@, that have no reason to
-- need @libjvm@ at link time); the caller resolves both SHAs via
-- 'DMML.Jgit.jgitResolve' and passes the results in.
--
-- Decision, exactly mirroring the bash original: if a checkpoint file
-- already exists at @checkpoints\/\<parentTreeSha\>.json@, fold ONLY
-- @newFiles@ into it. Otherwise -- no parent tree SHA at all (the very
-- first commit), or one that doesn't have a checkpoint file yet (a
-- prior checkpoint attempt failed, or history predates this mechanism)
-- -- fold EVERY @*.dmml@ file directly under @commitsDir@ instead, so
-- nothing already-committed is silently missing from the first real
-- checkpoint. This is the self-healing property the bash version's own
-- comments named: a missing checkpoint, whatever the reason, always
-- triggers a full fold rather than leaving the chain broken.
resolveAndFoldCheckpoint
  :: FilePath          -- ^ checkpoints directory
  -> FilePath          -- ^ commits directory (bootstrap fallback's full scan)
  -> Maybe String      -- ^ parent tree sha, if any
  -> String            -- ^ new tree sha (the checkpoint file's own name)
  -> [FilePath]        -- ^ newly-incorporated files (used unless bootstrapping)
  -> IO (FilePath, Int) -- ^ (path written, count folded as real commits)
resolveAndFoldCheckpoint checkpointsDir commitsDir parentTreeSha newTreeSha newFiles = do
  parentSnap <- case parentTreeSha of
    Nothing -> pure emptySnapshot
    Just sha -> do
      let parentPath = checkpointsDir </> (sha <> ".json")
      exists <- doesFileExist parentPath
      if not exists
        then pure emptySnapshot
        else do
          raw <- BL.readFile parentPath
          case decodeCheckpoint raw of
            Nothing -> ioError (userError ("resolveAndFoldCheckpoint: failed to decode parent checkpoint " <> parentPath))
            Just ck -> pure (checkpointToSnapshot ck)

  parentCheckpointExists <- case parentTreeSha of
    Nothing -> pure False
    Just sha -> doesFileExist (checkpointsDir </> (sha <> ".json"))

  filesToFold <-
    if parentCheckpointExists
      then pure newFiles
      else do
        -- Bootstrap/self-heal: no known parent checkpoint (no parent
        -- tree sha at all, or one whose checkpoint file is missing --
        -- a prior attempt failed, or history predates this mechanism).
        -- Scan every real commit file under commitsDir instead, same
        -- as the bash original's `find commits -name '*.dmml' -type f
        -- | sort` -- otherwise whatever the missing checkpoint was
        -- supposed to already cover would be silently absent from this
        -- one too.
        names <- listDirectory commitsDir
        pure (sortOn id [commitsDir </> n | n <- names, ".dmml" `isSuffixOf` n])

  (newSnap, foldedCount) <- foldNewFiles parentSnap filesToFold
  let outPath = checkpointsDir </> (newTreeSha <> ".json")
      out = snapshotToCheckpoint (T.pack newTreeSha) newSnap
  BL.writeFile outPath (encodeCheckpoint out)
  pure (outPath, foldedCount)
