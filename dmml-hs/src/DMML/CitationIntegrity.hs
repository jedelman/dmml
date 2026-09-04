{-# LANGUAGE OverloadedStrings #-}

-- | Real citation-integrity checking for @consumes@ blocks --
-- jedelman/dmml#6, found while rewriting @dmml-agent-nucleus/GRAMMAR.md@
-- to make @dmml-hs@ canonical: the retired Rust crate's @graph.rs@
-- checked a @consumes@ citation's @cid@ against what had actually been
-- recorded as observed for that @uri@ (accepting a first sighting,
-- rejecting a later one that disagrees); @DMML.Materialize.applyConsume@
-- had no equivalent at all -- a citation naming a @cid@ nobody ever saw
-- was accepted exactly the same as a real one.
--
-- What "observed" means here, concretely, for two different cases:
--
-- 1. __A file actually present in this batch__ (whatever set of
--    @.dmml@ files a caller is checking together): its own real
--    identity, via 'DMML.LocalIdentity.localFileRef' recomputed from
--    its own exact bytes, is authoritative. Any citation naming that
--    same @local:<path>@ uri MUST cite the matching @cid@ -- this is a
--    real check, not first-citation-wins, because we independently
--    know the truth (we just read those bytes ourselves). This is
--    exactly how a citation gets INTO a real commit in the first place:
--    'DMML.Fire.renderFiredCommit' writes @fact <uri>#<cid>@ straight
--    from a 'DMML.Ast.StrongRef' built by 'localFileRef' on the same
--    @--world@\/@--machine@ files a caller passes to @fire-transition@.
-- 2. __A uri for a file not present in this batch__ (a citation to some
--    other repo's commit, or a peer's commit not materialized here):
--    there is nothing to independently check it against, so the FIRST
--    citation seen establishes what "the" cid for that uri is taken to
--    be; a later citation of the same uri under a different cid is
--    rejected as inconsistent. This is the same real, if weak,
--    first-writer-trust check the retired Rust crate had --
--    @written-world/SPEC.md@ already discloses its limit: a writer can
--    still poison a node's first-seen cid record, there is no
--    verification against real substrate content. Not solved here,
--    same as it was never solved there.
--
-- Deliberately NOT scoped here: 'DMML.Ast.ConsumeStrong' (a whole-record
-- strongRef, not a specific fact) still isn't applied by
-- 'DMML.Materialize.applyConsume' at all -- a real, separate, disclosed
-- gap (it references a whole record for provenance\/authorization
-- purposes, e.g. a Bridge half or a Pentacle grant, not a fact to
-- retract), out of scope for this specific issue. Its own 'StrongRef'
-- IS still checked here, for the same integrity reason a 'ConsumeFact'
-- citation is: whatever the citation claims about a uri's cid should be
-- internally consistent, whether or not anything downstream acts on it
-- yet.
module DMML.CitationIntegrity
  ( CidLedger
  , emptyCidLedger
  , seedObserved
  , CitationError (..)
  , checkCommit
  , checkCommits
  ) where

import Control.Monad (foldM)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)

import DMML.Ast

-- | @uri -> the cid it's taken to actually have@ -- either because a
-- file with that uri was independently read in this batch
-- ('seedObserved'), or because some earlier citation established it
-- first ('checkRef''s own fallback).
newtype CidLedger = CidLedger (Map Text Text)
  deriving (Eq, Show)

emptyCidLedger :: CidLedger
emptyCidLedger = CidLedger Map.empty

-- | Record a real, independently-known identity as authoritative for
-- its uri -- e.g. every file a caller is checking together, via its own
-- 'DMML.LocalIdentity.localFileRef'. Call this for every file in the
-- batch BEFORE 'checkCommits', so a citation naming one of them is
-- checked against the real thing, not just whichever citation happened
-- to come first.
seedObserved :: StrongRef -> CidLedger -> CidLedger
seedObserved ref (CidLedger m) = CidLedger (Map.insert (strongRefUri ref) (strongRefCid ref) m)

data CitationError = CitationCidMismatch
  { citationUri :: Text
  , citationExpectedCid :: Text
  -- ^ what the ledger already had on record (either a real, independently-
  -- observed identity, or an earlier citation's own claim).
  , citationActualCid :: Text
  -- ^ what this citation claims.
  }
  deriving (Eq, Show)

checkRef :: StrongRef -> CidLedger -> Either CitationError CidLedger
checkRef ref (CidLedger m) =
  case Map.lookup (strongRefUri ref) m of
    Nothing -> Right (CidLedger (Map.insert (strongRefUri ref) (strongRefCid ref) m))
    Just recordedCid
      | recordedCid == strongRefCid ref -> Right (CidLedger m)
      | otherwise -> Left (CitationCidMismatch (strongRefUri ref) recordedCid (strongRefCid ref))

consumeEntryRef :: ConsumeEntry -> StrongRef
consumeEntryRef (ConsumeStrong ref) = ref
consumeEntryRef (ConsumeFact fc) = factConsumeCommit fc

itemRefs :: CommitItem -> [StrongRef]
itemRefs (ItemConsumes cb) = map consumeEntryRef (consumesEntries cb)
itemRefs _ = []

-- | Checks every @consumes@ citation in one commit, in source order,
-- against (and updating) the ledger.
checkCommit :: CidLedger -> CommitStmt -> Either CitationError CidLedger
checkCommit ledger stmt = foldM (flip checkRef) ledger (concatMap itemRefs (commitItems stmt))

-- | Checks a whole batch of commits, in order, threading one ledger
-- through all of them -- so a citation in a LATER commit is checked
-- against what an EARLIER commit in the same batch already established,
-- same as the real crate's own cross-commit behavior.
checkCommits :: CidLedger -> [CommitStmt] -> Either CitationError CidLedger
checkCommits = foldM checkCommit
