{-# LANGUAGE OverloadedStrings #-}

-- | A real, controllable dial on how permissive DMML's one structurally
-- loose slot (a string-literal attribute value) is allowed to be --
-- built specifically to run the Plank 1 dose-response experiment named
-- in @dev-journal/2026-09-03-desiring-machines-thesis.md@: does prose-
-- injection into this slot respond to how tightly it's constrained the
-- way a blocked-production/displaced-discharge account predicts (the
-- attempt persists, or reroutes elsewhere, as the cap tightens), or does
-- it just cleanly shrink the way "the model is only ever as verbose as
-- the grammar happens to allow" predicts? This module is the check;
-- it makes no claim about which account is right.
--
-- Deliberately the smallest real check: does any fact's string-literal
-- value in this commit exceed a given character cap. No claim about
-- WHICH facts are "supposed" to be terse vs. descriptive -- that's a
-- judgment call left to whoever calls this with a particular cap value,
-- same razor 'DMML.SelfDeclaration' already applies to its own scope.
module DMML.StringCap
  ( OverlongFact (..)
  , overlongStringFacts
  ) where

import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

data OverlongFact = OverlongFact
  { overlongSubject :: Text
  , overlongPredicate :: Text
  , overlongLength :: Int
  }
  deriving (Eq, Show)

-- | Every fact in this commit whose string-literal value exceeds the
-- given character cap, in declaration order. A negative or zero cap is
-- a legal (if extreme) call -- it just means every non-empty string
-- literal is overlong, useful as the tightest real condition in the
-- dose-response sweep.
overlongStringFacts :: Int -> CommitStmt -> [OverlongFact]
overlongStringFacts cap stmt =
  [ OverlongFact (nodeRefText (factSubject f)) (predText (factPredicate f)) len
  | item <- commitItems stmt
  , ItemFact f <- [item]
  , ValueLiteral (LitString s) <- [factValue f]
  , let len = T.length s
  , len > cap
  ]
  where
    nodeRefText (NodeRef segs) = T.intercalate "/" segs
    predText RdfType = "a"
    predText (PredIdent t) = t
