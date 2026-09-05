{-# LANGUAGE OverloadedStrings #-}

-- | Real, no-LLM check that every node in a materialized world can
-- actually be described -- the discipline SPEC.md #139 flagged as
-- required once "the director is officially out of the picture" (a
-- fully deterministic world needs every describable thing to actually
-- BE describable, and nothing enforced that until now). A subject is
-- describable iff either of this session's own two real mechanisms
-- would find something to say about it:
--
--   1. A direct naming fact ('DMML.TemplateBank.resolvePath''s own
--      convention -- @name@\/@epithet@, or whatever predicates the
--      caller passes) is asserted on it. Singularities.
--   2. It equips a machine governing at least one of its own live
--      facts ('DMML.Governance.findGoverningMachine' succeeds for some
--      (subject, predicate) pair the subject actually has) --
--      'DMML.TemplateBank.resolveViaGoverningMachine''s path.
--      Relations\/processes.
--
-- Deliberately does NOT check that the governing machine's CURRENT
-- state itself has a further naming fact, or that a resolved name
-- isn't empty -- this checks REACHABILITY of a description mechanism,
-- not that today's specific content already produced good prose. A
-- real, disclosed scope limit, matching the same "prove the mechanism
-- holds" discipline as every other module in this pass.
module DMML.Describability
  ( DescribabilityReport (..)
  , checkDescribability
  ) where

import Data.List (nub)
import qualified Data.Map.Strict as Map
import Data.Maybe (isJust)
import Data.Text (Text)

import DMML.Governance (findGoverningMachine)
import DMML.Materialize (WorldSnapshot (..), currentValue)

data DescribabilityReport = DescribabilityReport
  { describableSubjects :: [Text]
  , undescribableSubjects :: [Text]
  }
  deriving (Eq, Show)

-- | @namingPredicates@: which direct predicates count as a naming fact
-- (this session's convention: @["name", "epithet"]@, but never
-- hardcoded here -- a world is free to use its own vocabulary). No
-- 'DMML.Ast.MachineStmt' map needed -- 'findGoverningMachine' works
-- entirely off @equips@\/@trigger@ FACTS already in the snapshot, never
-- off a machine's own declaration, so this check only needs the
-- materialized world, the same as 'DMML.TemplateBank' itself.
checkDescribability :: [Text] -> WorldSnapshot -> DescribabilityReport
checkDescribability namingPredicates snap =
  DescribabilityReport
    { describableSubjects = [s | s <- subjects, isDescribable s]
    , undescribableSubjects = [s | s <- subjects, not (isDescribable s)]
    }
  where
    subjects = nub [subj | (subj, _pred) <- Map.keys (snapshotFacts snap)]
    predicatesOf subj = nub [p | (s, p) <- Map.keys (snapshotFacts snap), s == subj]
    hasDirectName subj = any (\p -> not (null (currentValue (subj, p) snap))) namingPredicates
    isGoverned subj = any (\p -> isJust (findGoverningMachine (subj, p) snap)) (predicatesOf subj)
    isDescribable subj = hasDirectName subj || isGoverned subj
