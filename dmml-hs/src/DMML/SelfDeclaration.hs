{-# LANGUAGE OverloadedStrings #-}

-- | Closed-vocabulary self-declaration checking -- a real, disclosed gap
-- this project has carried since @sync-spike/README.md@ first named it
-- ("does NOT check self-declaration... `validate.rs`\/`interpret.rs`
-- aren't ported to `dmml-hs` yet"), and which the de-prose operator's
-- first real run (2026-09-03) turned from a disclosed theoretical gap
-- into a real, silent content bug: an LLM extraction used
-- @forge \`locatedIn\` ashgrove@ while only ever declaring
-- @locatedOn@ -- accepted outright by @validate-commit@ (shape-only)
-- and untouched by @DMML.Retroconsistency.gateConsistentTree@ (which
-- only checks negated-guard consistency, a different property
-- entirely). No existing gate in this codebase checked this at all.
--
-- Deliberately the smallest real check, not a port of the production
-- crate's own two-pass `validate_self_declared` (`written-world`
-- `SPEC.md`'s own note on that being a two-pass check): every fact's
-- predicate must be a key in the snapshot's own 'DMML.Materialize.snapshotDeclared'
-- map. Nothing more -- no attempt to check WHICH commit declared it, or
-- whether the declared kind (relation vs. attribute) matches how the
-- predicate is actually used; those are real, separate checks, not
-- built here.
module DMML.SelfDeclaration
  ( undeclaredPredicates
  ) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)

import DMML.Materialize (WorldSnapshot (..))

-- | Every predicate used in a live fact that was never declared,
-- deduplicated and order-preserving by first occurrence. Empty means
-- the snapshot is fully self-declared.
--
-- Exempts @"a"@ -- Turtle-style @rdf:type@ sugar ('DMML.Ast.RdfType'),
-- not a user predicate at all ('DMML.Materialize.predText' renders it
-- as the literal string "a" with no way to tell it apart from a
-- 'PredIdent' spelled the same once materialized) -- confirmed against
-- the real 200-commit E1 endurance corpus: 30 real @. a = @ type-facts,
-- not one preceded by a @declare relation a@, accepted throughout by
-- every existing tool. Flagging it here would be a false positive on
-- this checker's part, not a real finding.
undeclaredPredicates :: WorldSnapshot -> [Text]
undeclaredPredicates snap =
  dedupe
    [ pred_
    | (_subj, pred_) <- Map.keys (snapshotFacts snap)
    , pred_ /= "a"
    , not (Map.member pred_ (snapshotDeclared snap))
    ]
  where
    dedupe = go []
    go seen [] = reverse seen
    go seen (x : xs)
      | x `elem` seen = go seen xs
      | otherwise = go (x : seen) xs
