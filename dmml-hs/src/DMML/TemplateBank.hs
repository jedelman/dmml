{-# LANGUAGE OverloadedStrings #-}

-- | A real, no-LLM prototype of SPEC.md §19.4's "text as closed-set
-- selection, not open generation" design (@written-world#139@): a
-- template is eligible for a subject iff (1) the subject is actually
-- declared to be of the template's own 'templateType' (checked via the
-- real, already-shipped @a@\/rdf:type fact, 'DMML.Ast.RdfType' --
-- Jason's 2026-09-04 correction: "templates should be bound to types")
-- and (2) every one of its declared coverage tags is actually true for
-- that subject in the given 'WorldSnapshot' -- both pure subset\/
-- membership checks, the same shape as the sprite-catalog check that
-- section describes, applied to text. Binding to type first is what
-- stops two entities that happen to share one attribute value but are
-- different KINDS of thing from cross-matching the same template.
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

import DMML.Ast (NodeRef, Value (ValueNode))
import DMML.Materialize (WorldSnapshot, currentValue)

-- | Every template is bound to exactly one 'templateType' -- checked
-- against the subject's own declared @subject :: a Type@ fact (Turtle-
-- style @rdf:type@ sugar, 'DMML.Ast.RdfType', already real, shipped
-- grammar -- 'DMML.Surface.pTypeOf' -- not a new construct this module
-- introduces). This is a deliberate tightening past a flat
-- @templateCovers@ tag bag: a template declares WHAT KIND of thing it
-- is for before it declares which conditions unlock it, so two entities
-- that happen to share an attribute value but are different kinds of
-- thing can never cross-match. Real, concrete case this prevents: a
-- @condition = corroded@ tag alone would match any entity with that
-- fact, including one for which "corroded" makes no sense (a spirit, a
-- person) -- binding the template to a type like @MetalObject@ makes
-- that cross-match structurally impossible, not just unlikely. See
-- @examples/template-compose-demo@'s `creature/wraith` for the actual
-- proof (a `Spirit` sharing `condition = corroded` with a
-- `MetalObject`, correctly excluded).
--
-- 'templateCovers': ADDITIONAL @(predicate, value)@ facts that must ALL
-- also be true, scoped within the type match above -- unchanged from
-- before, just no longer doing the type-discrimination job alone.
-- 'templateText': fixed, hand-vetted prose containing the literal
-- substring @{subject}@ where the subject's own name is substituted --
-- no other placeholder syntax, no interpolation of any fact value, so
-- there is no way for template rendering to surface a value the
-- eligibility check didn't already verify.
data Template = Template
  { templateId :: Text
  , templateType :: NodeRef
  , templateCovers :: [(Text, Value)]
  , templateText :: Text
  }
  deriving (Eq, Show)

-- | A template is eligible for @subject@ iff its declared type matches
-- a real @a@ (rdf:type) fact for that subject AND every one of its
-- coverage tags matches a real, currently-live fact -- 'currentValue'
-- already returns every live alternative for a (subject, predicate)
-- pair (DMML.Materialize's collision-free mints), so a tag matches if
-- the required value is among them. All conditions must match (a
-- template is never selected on a partial match) -- the deliberately
-- strict, deterministic membership check SPEC.md #139 describes as
-- replacing per-turn NLI, now gated by type first.
--
-- The rdf:type predicate's materialized key is the literal string
-- @"a"@ (`DMML.Materialize.predText`'s own encoding of `DMML.Ast.
-- RdfType` -- inlined here rather than imported, matching this
-- codebase's existing convention of several modules each defining
-- their own tiny `predText`-shaped helper rather than sharing one).
eligibleTemplates :: WorldSnapshot -> Text -> [Template] -> [Template]
eligibleTemplates snap subject = filter isEligible
  where
    isEligible tpl =
      ValueNode (templateType tpl) `elem` map snd (currentValue (subject, "a") snap)
        && all tagHolds (templateCovers tpl)
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
