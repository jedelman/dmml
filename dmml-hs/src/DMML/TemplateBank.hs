{-# LANGUAGE OverloadedStrings #-}

-- | A real, no-LLM prototype of SPEC.md §19.4's "text as closed-set
-- selection, not open generation" design (@written-world#139@).
--
-- CORRECTED 2026-09-04 (Jason: "the ENTIRE existing grammar is ALREADY
-- our field/domain... attributes are just facts are just DMML"): the
-- first version of this module invented its own eligibility mechanism
-- -- a bespoke @[(Text, Value)]@ tag list, checked by hand-written
-- Haskell matching logic, with a special-cased @templateType@ field
-- bolted on for type-binding. That was exactly the mistake this
-- project's own "DMML first" standing rule warns against (@CLAUDE.md@,
-- 2026-08-20): reaching for a new, narrower construct before checking
-- what the real grammar already says. A template's eligibility
-- condition is not a new kind of thing -- it is a guard, literally,
-- the same 'DMML.Ast.GuardClause'\/'DMML.Ast.ExistsExpr'\/'DMML.Ast.
-- Pattern' every machine transition already uses, evaluated by the
-- same, already-shipped, already-tested 'DMML.Guard.evalGuards'. Type-
-- binding was never a special case either -- @a MetalObject@ is just
-- another guard clause over the real @a@\/rdf:type fact, no different
-- in kind from @condition = corroded@. Collapsing both into one
-- mechanism isn't a simplification bought at the cost of expressiveness
-- -- it's a real GAIN: templates now get negation (@guard not ...@) and
-- arbitrary multi-hop patterns for free, because that's what
-- 'DMML.Guard' already supports and the old bespoke tag list never
-- could.
--
-- Deliberately does NOT call an LLM anywhere. The catalog itself
-- ('Template') is closed, curated content -- exactly the "base sprite/
-- decorator" precedent, just text instead of pixels. Slot-fill
-- ('renderTemplate') substitutes only the subject's own name; nothing
-- about template SELECTION or RENDERING is generative, so there is
-- nothing for a reality-check adversary to catch at this stage -- the
-- adversary's job (does this content assert more than its declared
-- guards support) moved entirely to catalog-authoring time, when a
-- human wrote 'templateText' to match 'templateGuards' by hand. This
-- module is the runtime half of that design; the governed catalog-
-- expansion pipeline (gap-detection guard -> request effect -> offline
-- generation -> review gate -> approved template) that would grow this
-- catalog is NOT built here -- this proves the runtime selection
-- mechanism holds, same "prove the shape before scoping the rest"
-- discipline as every other DMML-first example in this repo.
module DMML.TemplateBank
  ( Template (..)
  , eligibleTemplates
  , renderTemplate
  ) where

import Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast (GuardClause)
import DMML.Guard (EvalContext (..), evalGuards)
import DMML.Materialize (WorldSnapshot)

-- | 'templateGuards': the real guard list an eligible subject must
-- satisfy -- literally 'DMML.Ast.GuardClause' values, the exact type a
-- machine's own 'DMML.Ast.TransitionDecl' carries, evaluated the exact
-- same way ('DMML.Guard.evalGuards', plain conjunction). A "bound to
-- type" requirement is nothing special: just one more clause,
-- @EXISTS(self \`a\` SomeType)@, alongside whatever attribute clauses
-- also apply. 'templateText': fixed, hand-vetted prose containing the
-- literal substring @{subject}@ where the subject's own name is
-- substituted -- no other placeholder syntax, no interpolation of any
-- fact value, so there is no way for template rendering to surface a
-- value the guard check didn't already verify.
data Template = Template
  { templateId :: Text
  , templateGuards :: [GuardClause]
  , templateText :: Text
  }

-- | A template is eligible for @subject@ iff its real guards all hold
-- against @snap@, from @subject@'s own point of view -- 'EvalContext'
-- sets @subject@ as @self@ (exactly what a machine's own transitions
-- resolve @self@ against when firing), no params needed since template
-- guards never reference @$param@. This is not a reimplementation of
-- guard evaluation -- it IS 'DMML.Guard.evalGuards', unmodified.
eligibleTemplates :: WorldSnapshot -> Text -> [Template] -> [Template]
eligibleTemplates snap subject = filter isEligible
  where
    ctx = EvalContext {ctxSelfNode = subject, ctxParams = Map.empty}
    isEligible tpl = evalGuards (templateGuards tpl) ctx snap

-- | Deterministic slot-fill: the only substitution is the subject's own
-- display name into the literal @{subject}@ marker. Real DMML, no
-- interpretation -- this is code, not a model call.
renderTemplate :: Text -> Template -> Text
renderTemplate subjectDisplayName tpl =
  substitute "{subject}" subjectDisplayName (templateText tpl)
  where
    substitute needle replacement haystack
      | needle `isInfixOfT` haystack =
          let (before, after) = T.breakOn needle haystack
           in before <> replacement <> substitute needle replacement (T.drop (T.length needle) after)
      | otherwise = haystack
    isInfixOfT n h = T.unpack n `isInfixOf` T.unpack h
