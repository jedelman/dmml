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
  ) where

import Data.Aeson (FromJSON, ToJSON, decode, encode)
import qualified Data.ByteString.Lazy as BL
import Data.List (sortOn)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import GHC.Generics (Generic)

import DMML.Ast (DeclKind (..), Literal (..), NodeRef (..), Span (..), StrongRef (..), Value (..))
import DMML.Materialize (Alternatives (..), WorldSnapshot (..), alternativeEntries, emptySnapshot)

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
