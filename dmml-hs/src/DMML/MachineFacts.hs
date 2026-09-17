{-# LANGUAGE OverloadedStrings #-}

-- | Phase 1 of "machines and facts, unified" (Jason: "I'd like machines
-- to have the same guards as facts -- desiring machines, remember?").
-- A real, bidirectional encoding between 'MachineStmt' and flat fact
-- lists -- proof that a machine's structure can be SAID entirely as
-- facts, before anything about @DMML.Guard@\/@DMML.Fire@\'s actual
-- dispatch is touched. Deliberately scoped no further than that: this
-- module knows nothing about 'DMML.Materialize.WorldSnapshot',
-- multi-commit assertion, or firing -- it operates on a plain
-- @[FactStmt]@, the smallest thing that could prove the encoding
-- itself is sound. Wiring @mayFire@\/@fireTransition@ to read a
-- machine's structure FROM a snapshot's live facts, instead of from an
-- already-parsed 'MachineStmt' record, is real, separate, larger work
-- this phase deliberately does not attempt.
--
-- ONE hard constraint this design had to fit inside, confirmed by
-- actually parsing real content (see @.claude/skills/dmml-authoring@):
-- a single commit can never assert the same (subject, predicate) key
-- twice, even with different objects -- "the second occurrence would
-- silently overwrite the first." A machine has MANY states, MANY
-- transitions, MANY guards per transition -- so @hasState@\/
-- @hasTransition@\/@hasGuard@\/@hasEffect@ are all genuinely
-- multi-valued relations. That is not a problem to solve with a
-- linked-list encoding: DMML already has a first-class concept for
-- exactly this shape -- multiple LIVE ALTERNATIVES for one (subject,
-- predicate) key, the same thing 'DMML.Fire.FireRetractAmbiguous'
-- already reasons about -- so long as each element is asserted in a
-- SEPARATE commit. This module doesn't emit commits (that's a later
-- phase too), so it just emits one @hasX@ fact per element and leaves
-- "group these into separate commits at authoring time" as the real
-- constraint whatever renders this into Surface text must respect.
--
-- Every minted sub-node (a transition, a guard, a hop, an effect) is
-- named by extending the machine's own node with a deterministic
-- suffix path -- open-world minting, the same mechanism
-- 'DMML.Ast.Effect'\'s own doc comment already describes for an
-- ordinary assert.
--
-- ORDER: a machine's own state\/transition\/guard\/effect lists are
-- never semantically ordered (states are just names; guards are
-- ANDed; which assert/retract effect object fires first doesn't change
-- what a transition ultimately asserts, since 'DMML.Fire.gateCheck'
-- gates the whole resolved set at once) -- so 'decodeMachine' does NOT
-- guarantee the original authoring order of any of those comes back,
-- only their content (verified by the round-trip test comparing
-- SORTED lists, not raw equality). The one place order IS semantically
-- real is a guard or retract's own HOP sequence (a multi-hop pattern
-- walks forward, hop by hop) -- that gets an explicit numeric
-- @hopIndex@ fact so decode can restore the real sequence regardless
-- of what order the facts themselves were read in.
module DMML.MachineFacts
  ( encodeMachine
  , decodeMachine
  , DecodeError (..)
  ) where

import Data.List (sortOn)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

sp :: Span
sp = Span "/machine-facts"

-- ---------------------------------------------------------------------
-- node naming
-- ---------------------------------------------------------------------

childNode :: NodeRef -> [Text] -> NodeRef
childNode (NodeRef segs) suffix = NodeRef (segs ++ suffix)

-- ---------------------------------------------------------------------
-- term <-> text (duplicated in miniature from DMML.Fire's
-- renderPatternTerm/pPatternTerm's own logic, not imported from
-- DMML.Fire -- Fire.hs pulls in DMML.Surface, which needs megaparsec,
-- and this module is deliberately kept buildable with base packages
-- only, the same reason DMML.SpawnCycles is)
-- ---------------------------------------------------------------------

-- | NOT the lossy surface spelling ('DMML.Fire.renderPatternTerm's
-- job, a real, DISCLOSED, different concern -- that one has to match
-- what @DMML.Surface@ can re-parse, single-segment ambiguity and all).
-- This module never goes through the parser, so it owes real grammar
-- text nothing -- an explicit tag beats reusing a spelling that can't
-- tell 'TermNode' from 'TermVar' once a node reference happens to be
-- single-segment. Caught by this module's own round-trip test: the
-- state-sugar desugaring ('DMML.Ast.Effect'\'s own doc comment: a bare
-- @assert unlocked@ lowers to @EffectValueTerm (TermNode "unlocked")@)
-- produces exactly this single-segment-'TermNode' shape for real, and
-- the lossy text encoding silently turned it into a 'TermVar' on
-- decode -- a real bug the test caught before this shipped, not a
-- hypothetical one.
termToText :: PatternTerm -> Text
termToText TermSelf = "self"
termToText (TermParam p) = "$" <> p
termToText (TermNode n) = "node:" <> n
termToText (TermVar v) = "var:" <> v

termFromText :: Text -> PatternTerm
termFromText t
  | t == "self" = TermSelf
  | Just p <- T.stripPrefix "$" t = TermParam p
  | Just n <- T.stripPrefix "node:" t = TermNode n
  | Just v <- T.stripPrefix "var:" t = TermVar v
  | otherwise = TermVar t -- unreachable via encodeMachine; kept total rather than partial

-- ---------------------------------------------------------------------
-- fact-building helpers
-- ---------------------------------------------------------------------

fact :: NodeRef -> Text -> Value -> FactStmt
fact subj pred_ val = FactStmt {factSubject = subj, factPredicate = PredIdent pred_, factValue = val, factSpan = sp}

nodeF :: NodeRef -> Text -> NodeRef -> FactStmt
nodeF subj pred_ obj = fact subj pred_ (ValueNode obj)

strF :: NodeRef -> Text -> Text -> FactStmt
strF subj pred_ s = fact subj pred_ (ValueLiteral (LitString s))

numF :: NodeRef -> Text -> Int -> FactStmt
numF subj pred_ n = fact subj pred_ (ValueLiteral (LitNumber (T.pack (show n))))

boolF :: NodeRef -> Text -> Bool -> FactStmt
boolF subj pred_ b = fact subj pred_ (ValueLiteral (LitBoolean b))

-- ---------------------------------------------------------------------
-- encode
-- ---------------------------------------------------------------------

-- | Every fact this machine's structure can be said as -- spread this
-- across separate commits before asserting it for real; this function
-- only produces the facts, it makes no claim about how they're
-- grouped into commits.
encodeMachine :: MachineStmt -> [FactStmt]
encodeMachine m =
  concatMap encodeState (machineStates m)
    ++ concatMap encodeTransition (machineTransitions m)
  where
    mNode = machineNode m

    encodeState s =
      let sNode = childNode mNode ["st", stateIdent s]
       in [nodeF mNode "hasState" sNode, strF sNode "stateName" (stateIdent s)]

    encodeTransition t =
      let tNode = childNode mNode ["tr", transitionIdent t]
       in [nodeF mNode "hasTransition" tNode, strF tNode "transitionName" (transitionIdent t)]
            ++ [nodeF tNode "fromState" (childNode mNode ["st", f]) | f <- maybe [] pure (transitionFrom t)]
            ++ [nodeF tNode "toState" (childNode mNode ["st", to]) | to <- maybe [] pure (transitionTo t)]
            ++ concatMap (encodeParam tNode) (transitionParams t)
            ++ concat [encodeGuard tNode gi g | (gi, g) <- zip [0 :: Int ..] (transitionGuards t)]
            ++ concat [encodeEffect tNode ei e | (ei, e) <- zip [0 :: Int ..] (transitionEffects t)]

    encodeParam tNode p =
      let pNode = childNode tNode ["p", p]
       in [nodeF tNode "hasParam" pNode, strF pNode "paramName" p]

    encodeGuard tNode gi g =
      let gNode = childNode tNode ["g", T.pack (show gi)]
          pat = existsPattern (guardExists g)
       in [ nodeF tNode "hasGuard" gNode
          , boolF gNode "guardNegated" (guardNegated g)
          , strF gNode "guardAnchor" (termToText (patternAnchor pat))
          ]
            ++ concat [encodeHop gNode hi h | (hi, h) <- zip [0 :: Int ..] (patternHops pat)]

    encodeHop ownerNode hi h =
      let hNode = childNode ownerNode ["h", T.pack (show hi)]
       in [ nodeF ownerNode "hasHop" hNode
          , numF hNode "hopIndex" hi
          , strF hNode "hopPredicate" (hopPredicate h)
          , strF hNode "hopTerm" (termToText (hopTerm h))
          ]

    encodeEffect tNode ei eff =
      let eNode = childNode tNode ["e", T.pack (show ei)]
          common = [nodeF tNode "hasEffect" eNode, numF eNode "effectIndex" ei]
       in common ++ case eff of
            EffectAssert subj predRef val ->
              [ strF eNode "effectKind" "assert"
              , strF eNode "effectSubject" (termToText subj)
              , strF eNode "effectPredicate" (predText predRef)
              ]
                ++ encodeEffectValue eNode val
            EffectRetract subj hops predRef mVal ->
              [ strF eNode "effectKind" "retract"
              , strF eNode "effectSubject" (termToText subj)
              , strF eNode "effectPredicate" (predText predRef)
              , boolF eNode "effectHasValue" (maybe False (const True) mVal)
              ]
                ++ concat [encodeHop eNode hi h | (hi, h) <- zip [0 :: Int ..] hops]
                ++ maybe [] (encodeEffectValue eNode) mVal
            EffectSpawn newNodeTerm templateRef ->
              [ strF eNode "effectKind" "spawn"
              , strF eNode "effectSubject" (termToText newNodeTerm)
              , strF eNode "effectTemplate" (T.intercalate "/" (nodeRefSegments templateRef))
              ]

    encodeEffectValue eNode val = case val of
      EffectValueTerm t -> [strF eNode "effectValueKind" "term", strF eNode "effectValueText" (termToText t)]
      EffectValueLiteral (LitString s) -> [strF eNode "effectValueKind" "string", strF eNode "effectValueText" s]
      EffectValueLiteral (LitNumber n) -> [strF eNode "effectValueKind" "number", strF eNode "effectValueText" n]
      EffectValueLiteral (LitBoolean b) ->
        [strF eNode "effectValueKind" "bool", strF eNode "effectValueText" (if b then "true" else "false")]

    predText RdfType = "a"
    predText (PredIdent p) = p

-- ---------------------------------------------------------------------
-- decode
-- ---------------------------------------------------------------------

data DecodeError
  = MissingFact NodeRef Text
  | MalformedFact NodeRef Text Text
  deriving (Eq, Show)

-- | Groups facts by (subject, predicate) once, up front, so every
-- lookup below is a map lookup, not a fresh linear scan -- the facts
-- list a real caller hands in could be a whole world snapshot's worth,
-- not just one machine's.
type FactIndex = Map.Map (Text, Text) [Value]

buildIndex :: [FactStmt] -> FactIndex
buildIndex facts =
  Map.fromListWith
    (++)
    [ ((nodeText (factSubject f), predName (factPredicate f)), [factValue f])
    | f <- facts
    ]
  where
    predName RdfType = "a"
    predName (PredIdent p) = p

nodeText :: NodeRef -> Text
nodeText = T.intercalate "/" . nodeRefSegments

lookupMany :: FactIndex -> NodeRef -> Text -> [Value]
lookupMany idx subj pred_ = Map.findWithDefault [] (nodeText subj, pred_) idx

lookupNodes :: FactIndex -> NodeRef -> Text -> [NodeRef]
lookupNodes idx subj pred_ = [NodeRef (T.splitOn "/" (nodeText n)) | ValueNode n <- lookupMany idx subj pred_]

lookupOneStr :: FactIndex -> NodeRef -> Text -> Either DecodeError Text
lookupOneStr idx subj pred_ = case [s | ValueLiteral (LitString s) <- lookupMany idx subj pred_] of
  (s : _) -> Right s
  [] -> Left (MissingFact subj pred_)

lookupOneBool :: FactIndex -> NodeRef -> Text -> Either DecodeError Bool
lookupOneBool idx subj pred_ = case [b | ValueLiteral (LitBoolean b) <- lookupMany idx subj pred_] of
  (b : _) -> Right b
  [] -> Left (MissingFact subj pred_)

lookupOneInt :: FactIndex -> NodeRef -> Text -> Either DecodeError Int
lookupOneInt idx subj pred_ = case [n | ValueLiteral (LitNumber n) <- lookupMany idx subj pred_] of
  (n : _) -> maybe (Left (MalformedFact subj pred_ n)) Right (readMaybeInt n)
  [] -> Left (MissingFact subj pred_)
  where
    readMaybeInt s = case reads (T.unpack s) of
      [(v, "")] -> Just v
      _ -> Nothing

lookupOneNode :: FactIndex -> NodeRef -> Text -> Either DecodeError NodeRef
lookupOneNode idx subj pred_ = case lookupNodes idx subj pred_ of
  (n : _) -> Right n
  [] -> Left (MissingFact subj pred_)

-- | Reconstructs a 'MachineStmt' from every fact naming @machineNode@
-- (directly or transitively, via the minted sub-nodes 'encodeMachine'
-- itself produces) -- the exact inverse of 'encodeMachine', modulo
-- list order (see this module's own doc comment on why order was
-- never semantically real for anything but a hop sequence).
decodeMachine :: NodeRef -> [FactStmt] -> Either DecodeError MachineStmt
decodeMachine mNode facts = do
  let idx = buildIndex facts
  states <- traverse (decodeState idx) (lookupNodes idx mNode "hasState")
  transitions <- traverse (decodeTransition idx) (lookupNodes idx mNode "hasTransition")
  pure MachineStmt {machineNode = mNode, machineStates = states, machineTransitions = transitions, machineSpan = sp}
  where
    decodeState idx sNode = do
      name <- lookupOneStr idx sNode "stateName"
      pure StateDecl {stateIdent = name, stateSpan = sp}

    decodeTransition idx tNode = do
      name <- lookupOneStr idx tNode "transitionName"
      let mFrom = either (const Nothing) (Just . lastSeg) (lookupOneNode idx tNode "fromState")
          mTo = either (const Nothing) (Just . lastSeg) (lookupOneNode idx tNode "toState")
      params <- traverse (\pNode -> lookupOneStr idx pNode "paramName") (lookupNodes idx tNode "hasParam")
      guards <- traverse (decodeGuard idx) (lookupNodes idx tNode "hasGuard")
      effects <- decodeEffectsInOrder idx tNode
      pure
        TransitionDecl
          { transitionIdent = name
          , transitionParams = params
          , transitionFrom = mFrom
          , transitionTo = mTo
          , transitionGuards = guards
          , transitionEffects = effects
          , transitionSpan = sp
          }

    lastSeg n = case nodeRefSegments n of
      [] -> ""
      xs -> last xs

    decodeGuard idx gNode = do
      neg <- lookupOneBool idx gNode "guardNegated"
      anchorText <- lookupOneStr idx gNode "guardAnchor"
      hops <- decodeHopsInOrder idx gNode
      let pat = Pattern {patternAnchor = termFromText anchorText, patternHops = hops}
      pure GuardClause {guardNegated = neg, guardExists = ExistsExpr {existsPattern = pat, existsSpan = sp}, guardSpan = sp}

    decodeHopsInOrder idx ownerNode = do
      let hNodes = lookupNodes idx ownerNode "hasHop"
      indexed <- traverse (\hNode -> (,) hNode <$> lookupOneInt idx hNode "hopIndex") hNodes
      traverse (decodeHop idx . fst) (sortOn snd indexed)

    decodeHop idx hNode = do
      p <- lookupOneStr idx hNode "hopPredicate"
      termText <- lookupOneStr idx hNode "hopTerm"
      pure PatternHop {hopPredicate = p, hopTerm = termFromText termText}

    decodeEffectsInOrder idx tNode = do
      let eNodes = lookupNodes idx tNode "hasEffect"
      indexed <- traverse (\eNode -> (,) eNode <$> lookupOneInt idx eNode "effectIndex") eNodes
      traverse (decodeEffect idx . fst) (sortOn snd indexed)

    decodeEffect idx eNode = do
      kind <- lookupOneStr idx eNode "effectKind"
      subjText <- lookupOneStr idx eNode "effectSubject"
      case kind of
        "assert" -> do
          predName <- lookupOneStr idx eNode "effectPredicate"
          val <- decodeEffectValue idx eNode
          pure (EffectAssert (termFromText subjText) (PredIdent predName) val)
        "retract" -> do
          predName <- lookupOneStr idx eNode "effectPredicate"
          hasVal <- lookupOneBool idx eNode "effectHasValue"
          hops <- decodeHopsInOrder idx eNode
          mVal <- if hasVal then Just <$> decodeEffectValue idx eNode else pure Nothing
          pure (EffectRetract (termFromText subjText) hops (PredIdent predName) mVal)
        "spawn" -> do
          templateText <- lookupOneStr idx eNode "effectTemplate"
          pure (EffectSpawn (termFromText subjText) (NodeRef (T.splitOn "/" templateText)))
        other -> Left (MalformedFact eNode "effectKind" other)

    decodeEffectValue idx eNode = do
      kind <- lookupOneStr idx eNode "effectValueKind"
      txt <- lookupOneStr idx eNode "effectValueText"
      case kind of
        "term" -> Right (EffectValueTerm (termFromText txt))
        "string" -> Right (EffectValueLiteral (LitString txt))
        "number" -> Right (EffectValueLiteral (LitNumber txt))
        "bool" -> Right (EffectValueLiteral (LitBoolean (txt == "true")))
        other -> Left (MalformedFact eNode "effectValueKind" other)
