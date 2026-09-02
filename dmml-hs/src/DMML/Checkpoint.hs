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

import DMML.Ast (DeclKind (..), Literal (..), NodeRef (..), Value (..))
import DMML.Materialize (Alternatives (..), WorldSnapshot (..), emptySnapshot)

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

-- | One (subject, predicate) pair's live alternatives, list-shaped
-- rather than a 'Data.Map.Strict.Map' keyed on a tuple -- aeson has no
-- built-in 'Data.Aeson.ToJSONKey' for an arbitrary tuple, and this
-- avoids needing one. Same idiom 'EntropySidecar.hs'\'s own
-- @CheckpointPair@ already established for exactly this reason.
data CheckpointFact = CheckpointFact
  { cfSubject :: Text
  , cfPredicate :: Text
  , cfAlternatives :: [(Text, Value)]
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
        [ CheckpointFact subj pred_ (alternativeValues alts)
        | ((subj, pred_), alts) <- sortOn fst (Map.toList (snapshotFacts snap))
        ]
    }

checkpointToSnapshot :: CheckpointFile -> WorldSnapshot
checkpointToSnapshot ck =
  emptySnapshot
    { snapshotDeclared = Map.fromList (ckDeclared ck)
    , snapshotFacts =
        Map.fromList
          [ ((cfSubject f, cfPredicate f), Alternatives (cfAlternatives f))
          | f <- ckFacts ck
          ]
    }

encodeCheckpoint :: CheckpointFile -> BL.ByteString
encodeCheckpoint = encode

decodeCheckpoint :: BL.ByteString -> Maybe CheckpointFile
decodeCheckpoint = decode
