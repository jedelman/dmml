{-# LANGUAGE OverloadedStrings #-}

-- | A real, no-LLM prototype of SPEC.md §19.4's "text as closed-set
-- selection, not open generation" design (@written-world#139@): a
-- template is eligible for a subject iff every one of its declared
-- coverage tags is actually true for that subject in the given
-- 'WorldSnapshot' -- a pure subset\/membership check, the same shape as
-- the sprite-catalog check that section describes, applied to text.
--
-- Deliberately does NOT call an LLM anywhere. The catalog itself
-- ('Template') is closed, curated content -- exactly the "base sprite/
-- decorator" precedent, just text instead of pixels. Slot-fill
-- ('renderTemplate') substitutes only the subject's own name; nothing
-- about template SELECTION or RENDERING is generative, so there is
-- nothing for a reality-check adversary to catch at this stage -- the
-- adversary's job (does this content assert more than its declared
-- tags support) moved entirely to catalog-authoring time, when a human
-- wrote 'templateText' to match 'templateCovers' by hand. This module
-- is the runtime half of that design; the governed catalog-expansion
-- pipeline (gap-detection guard -> request effect -> offline generation
-- -> review gate -> approved template) that would grow this catalog is
-- NOT built here -- this proves the runtime selection mechanism holds,
-- same "prove the shape before scoping the rest" discipline as every
-- other DMML-first example in this repo.
module DMML.TemplateBank
  ( Template (..)
  , eligibleTemplates
  , renderTemplate
  ) where

import Data.List (isInfixOf)
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast (Value)
import DMML.Materialize (WorldSnapshot, currentValue)

-- | 'templateCovers': the exact set of @(predicate, value)@ facts that
-- must ALL be true of a subject for this template to be eligible.
-- 'templateText': fixed, hand-vetted prose containing the literal
-- substring @{subject}@ where the subject's own name is substituted --
-- no other placeholder syntax, no interpolation of any fact value, so
-- there is no way for template rendering to surface a value the
-- eligibility check didn't already verify.
data Template = Template
  { templateId :: Text
  , templateCovers :: [(Text, Value)]
  , templateText :: Text
  }
  deriving (Eq, Show)

-- | A template is eligible for @subject@ iff EVERY one of its coverage
-- tags matches a real, currently-live fact for that subject in
-- @snap@ -- 'currentValue' already returns every live alternative for a
-- (subject, predicate) pair (DMML.Materialize's collision-free mints),
-- so a tag matches if the required value is among them. All tags must
-- match (a template requiring two conditions is never selected on one
-- alone) -- the deliberately strict, deterministic membership check
-- SPEC.md #139 describes as replacing per-turn NLI.
eligibleTemplates :: WorldSnapshot -> Text -> [Template] -> [Template]
eligibleTemplates snap subject = filter isEligible
  where
    isEligible tpl = all tagHolds (templateCovers tpl)
    tagHolds (predicate, requiredValue) =
      requiredValue `elem` map snd (currentValue (subject, predicate) snap)

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
