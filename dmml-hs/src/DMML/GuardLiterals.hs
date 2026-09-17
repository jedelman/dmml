{-# LANGUAGE OverloadedStrings #-}

-- | Non-blocking, additive check for the single most-repeated real
-- authoring mistake this project has hit: a guard-pattern term with no
-- @\/@ is always an existential PATTERN VARIABLE (matches anything),
-- never a literal match, even when it happens to spell a real,
-- declared state name -- 'DMML.Surface.pPatternTerm's own doc comment
-- names this as a real, deliberate limitation ("this surface can't
-- tell them apart"), not a bug. Hit three separate times in one real
-- session authoring @examples\/jev-driver-demo@'s ecology scenario
-- (@guard $patch \`state\` lush@, @guard $target \`state\` hungry@ x2)
-- before this checker existed -- each one silently passed (matched
-- ANY state) instead of refusing, and the second occurrence was masked
-- by a DIFFERENT, unrelated refusal downstream that looked like the
-- guard had correctly failed. See @.claude\/skills\/dmml-authoring@'s
-- first documented landmine for the full account this checker exists
-- to catch mechanically instead of by discipline alone.
--
-- Deliberately the same shape as 'DMML.SelfDeclaration.undeclaredPredicates':
-- a narrow, high-confidence heuristic, not a general static analyzer.
-- It does not (and cannot) know authorial INTENT -- a bare guard term
-- that happens to collide with a declared state name is not
-- necessarily wrong, just suspicious enough to flag every time, the
-- same way @check-declared@ flags every undeclared predicate without
-- claiming to know whether the author meant to declare it.
module DMML.GuardLiterals
  ( SuspiciousTerm (..)
  , suspiciousGuardTerms
  ) where

import Data.Text (Text)

import DMML.Ast

-- | One bare (slash-free) guard-pattern term whose text exactly
-- matches a state name declared by at least one machine in the same
-- scanned set.
data SuspiciousTerm = SuspiciousTerm
  { stMachine :: NodeRef
  -- ^ The machine whose transition guards on this term.
  , stTransition :: Text
  , stVar :: Text
  -- ^ The bare term text itself (an existential variable at parse
  -- time, whatever the author may have intended).
  , stDeclaredBy :: [NodeRef]
  -- ^ Every machine (in the same scanned set) that declares a state
  -- with exactly this name -- what makes this term suspicious rather
  -- than an ordinary, correctly-open pattern variable.
  }
  deriving (Eq, Show)

-- | Scans every guard clause (anchor and every hop) of every
-- transition on every given machine. A hit requires nothing beyond
-- exact text equality between a bare 'TermVar' and some machine's
-- declared 'stateIdent' -- real false positives are possible (a
-- genuinely intentional open pattern variable that happens to share a
-- word with an unrelated state elsewhere), which is why this is a
-- warning-and-continue check (see @app\/CheckGuardLiterals.hs@), never
-- a hard parse-time rejection: the grammar's own behavior here is
-- correct and deliberate, this only flags where it is likely to have
-- surprised the author.
suspiciousGuardTerms :: [MachineStmt] -> [SuspiciousTerm]
suspiciousGuardTerms machines =
  [ SuspiciousTerm (machineNode m) (transitionIdent t) v declaredBy
  | m <- machines
  , t <- machineTransitions m
  , g <- transitionGuards t
  , let pat = existsPattern (guardExists g)
  , term <- patternAnchor pat : map hopTerm (patternHops pat)
  , TermVar v <- [term]
  , let declaredBy = [machineNode m' | m' <- machines, v `elem` map stateIdent (machineStates m')]
  , not (null declaredBy)
  ]
