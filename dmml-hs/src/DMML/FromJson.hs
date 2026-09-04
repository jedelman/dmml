{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ScopedTypeVariables #-}

-- | JSON -> 'DMML.Ast.*' construction, translated from the real Rust
-- @dmml@ crate's @src/from_json.rs@ line for line where the two
-- languages let it stay that close. This is DMML's only authoring
-- surface: there is no text grammar, so everything here builds AST
-- directly out of "DMML.Json"'s wire shapes, the same way the Rust
-- module does.
--
-- Design rules the shapes in "DMML.Json" all follow, so a tool-calling
-- agent's output is checked at the API boundary before it ever reaches
-- this code (copied verbatim from the Rust module doc, since they are
-- exactly as true here):
--
--   * One discriminant field name everywhere a shape is tagged: @kind@.
--   * Every tagged variant is distinguishable by that one field alone.
--   * Omitting a field always means the same thing every time it's
--     omittable (an empty list, or the 'FactRef' wildcard); @null@ is
--     never sent or expected.
--   * Node references and predicates stay plain 'Data.Text.Text', not
--     decomposed structs.
--
-- A 'DMML.Ast.Span' is a JSON Pointer (RFC 6901) into the request
-- payload this AST node came from, e.g. @\/facts\/2\/predicate@. Built
-- by hand at each construction site below, exactly as the Rust source
-- does, since the indices are already in hand while walking the input.
module DMML.FromJson
  ( JsonError (..)
  , FromJsonError (..)
  , UpdateFromJsonError (..)
  , commitFromJson
  , machineFromJson
  , referenceFromJson
  , updateFromJson
    -- * Exposed for testing/inspection, mirroring the Rust module's own
    -- public @*_stmt_from_input@ functions.
  , commitStmtFromInput
  , machineStmtFromInput
  , referenceStmtFromInput
  , isValidIdent
  , isValidNodeRef
  ) where

import Control.Monad (forM, forM_, unless, when)
import Data.Aeson (FromJSON, eitherDecode)
import qualified Data.ByteString.Lazy as BL
import Data.Char (isAsciiLower, isAsciiUpper, isDigit)
import Data.List (foldl')
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Maybe (isJust)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE

import DMML.Ast
import qualified DMML.Json as J

-- ---------------------------------------------------------------------
-- Errors
-- ---------------------------------------------------------------------

data JsonError = JsonError {jePointer :: Text, jeMessage :: Text}
  deriving (Eq, Show)

data FromJsonError
  = -- | The request body wasn't valid JSON, or didn't match the expected
    -- input shape at all (wrong type, missing required field).
    FJDecodeError String
  | -- | The JSON was shaped correctly, but a value inside it isn't valid
    -- DMML content (a malformed identifier, node reference; an empty
    -- commit).
    FJInvalid JsonError
  deriving (Eq, Show)

data UpdateFromJsonError
  = UFJDecodeError String
  | -- | Every entry's pointer is rebased onto the update (e.g.
    -- @\/update\/1\/commits\/3\/facts\/1\/subject@), not just the
    -- offending item's own local pointer.
    UFJInvalid [JsonError]
  deriving (Eq, Show)

invalid :: Text -> Text -> Either JsonError a
invalid pointer message = Left (JsonError pointer message)

tshow :: Show a => a -> Text
tshow = T.pack . show

-- | Rough stand-in for Rust's @{value:?}@ debug-quoted string
-- formatting, used only inside error messages.
dquote :: Text -> Text
dquote t = "\"" <> T.concatMap esc t <> "\""
  where
    esc '"' = "\\\""
    esc '\\' = "\\\\"
    esc c = T.singleton c

idx :: Text -> Int -> Text
idx prefix i = prefix <> "/" <> tshow i

-- ---------------------------------------------------------------------
-- Lexical checks
-- ---------------------------------------------------------------------

-- | A bare @ident@: letter-led, otherwise alphanumeric\/underscore.
isValidIdent :: Text -> Bool
isValidIdent s = case T.uncons s of
  Nothing -> False
  Just (c, rest) -> isAsciiAlpha c && T.all (\c' -> isAsciiAlphaNum c' || c' == '_') rest
  where
    isAsciiAlpha c = isAsciiLower c || isAsciiUpper c
    isAsciiAlphaNum c = isAsciiAlpha c || isDigit c

-- | @seg_piece@: @ident | number@, where @number@ here is the digit-led
-- form only (no leading @-@, no decimal part).
isValidSegPiece :: Text -> Bool
isValidSegPiece s = isValidIdent s || (not (T.null s) && T.all isDigit s)

-- | @node_ref@: @segment , { "\/" , segment }@ where
-- @segment = seg_piece , { "." , seg_piece }@ -- e.g. @room\/42@,
-- @key\/7@, @room\/42.reach@.
isValidNodeRef :: Text -> Bool
isValidNodeRef s =
  not (T.null s)
    && all (\segment -> not (T.null segment) && all isValidSegPiece (T.splitOn "." segment)) (T.splitOn "/" s)

checkIdent :: Text -> Text -> Either JsonError ()
checkIdent pointer value =
  if isValidIdent value
    then Right ()
    else invalid pointer (dquote value <> " is not a valid identifier")

nodeRef :: Text -> Text -> Either JsonError NodeRef
nodeRef pointer value =
  if isValidNodeRef value
    then Right (NodeRef (T.splitOn "/" value))
    else invalid pointer (dquote value <> " is not a valid node reference")

-- | @predicate_ref = "a" | ident@ -- the one place a bare, non-@ident@
-- token is legal on its own.
predicateRef :: Text -> Text -> Either JsonError PredicateRef
predicateRef pointer value
  | value == "a" = Right RdfType
  | otherwise = checkIdent pointer value >> Right (PredIdent value)

-- | A 'DMML.Ast.StrongRef' target URI: opaque, substrate-chosen, only
-- required to be non-empty. No @at:\/\/@ (or any other) scheme is
-- assumed here.
strongRefUriCheck :: Text -> Text -> Either JsonError Text
strongRefUriCheck pointer raw
  | T.null raw = invalid pointer "uri must not be empty"
  | otherwise = Right raw

strongRef :: Text -> J.StrongRefInput -> Either JsonError StrongRef
strongRef pointer input = do
  uri <- strongRefUriCheck (pointer <> "/uri") (J.sriUri input)
  pure StrongRef {strongRefUri = uri, strongRefCid = J.sriCid input, strongRefSpan = Span pointer}

valueFromInput :: Text -> J.ObjectInput -> Either JsonError Value
valueFromInput pointer input = case input of
  J.ObjNode v -> ValueNode <$> nodeRef (pointer <> "/value") v
  J.ObjStr v -> Right (ValueLiteral (LitString v))
  J.ObjNumber v -> Right (ValueLiteral (LitNumber v))
  J.ObjBoolean b -> Right (ValueLiteral (LitBoolean b))

-- ---------------------------------------------------------------------
-- CommitInput -> CommitStmt
-- ---------------------------------------------------------------------

-- | A repeated (subject, predicate) pair within one commit's own facts
-- is never something an agent means to do -- 'Materialized' keeps
-- exactly one current value per (subject, predicate), so the second
-- occurrence would silently overwrite the first with no signal at all.
-- Checked on the raw JSON strings (before node\/predicate validation has
-- necessarily run for every entry), so a duplicate is reported even
-- alongside other shape errors, not masked by them.
checkDuplicateFactsWithinCommit :: [J.FactInput] -> Either JsonError ()
checkDuplicateFactsWithinCommit facts = go Map.empty (zip [0 ..] facts)
  where
    go :: Map (Text, Text) Int -> [(Int, J.FactInput)] -> Either JsonError ()
    go _ [] = Right ()
    go seen ((i, f) : rest) =
      let key = (J.fiSubject f, J.fiPredicate f)
       in case Map.lookup key seen of
            Just firstI ->
              invalid
                (idx "/facts" i)
                ( "duplicate ("
                    <> J.fiSubject f
                    <> ", "
                    <> J.fiPredicate f
                    <> ") -- already asserted at /facts/"
                    <> tshow firstI
                    <> "; the second occurrence would silently overwrite the first"
                )
            Nothing -> go (Map.insert key i seen) rest

-- | Builds a 'CommitStmt' directly from a 'J.CommitInput' -- no source
-- text is ever produced. 'J.ciFacts' entries become bare 'ItemFact'
-- items (the "implicit produces block" sugar), matching how every
-- existing JSON-authored commit has always been shaped.
commitStmtFromInput :: J.CommitInput -> Either JsonError CommitStmt
commitStmtFromInput input = do
  checkIdent "/verb" (J.ciVerb input)
  when
    (null (J.ciFacts input) && null (J.ciConsumes input) && all null (Map.elems (J.ciRefs input)))
    (invalid "" "commit has no facts, consumes, or refs")

  declareItems <- forM (zip [0 ..] (J.ciDeclares input)) $ \(i, decl) -> do
    let pointer = idx "/declares" i
    checkIdent (pointer <> "/name") (J.diName decl)
    pure
      ( ItemDeclare
          DeclareStmt
            { declareKind = case J.diKind decl of J.DeclareRelation -> DeclRelation; J.DeclareAttribute -> DeclAttribute
            , declareIdent = J.diName decl
            , declareSpan = Span pointer
            }
      )

  checkDuplicateFactsWithinCommit (J.ciFacts input)

  factItems <- forM (zip [0 ..] (J.ciFacts input)) $ \(i, fact) -> do
    let pointer = idx "/facts" i
    subj <- nodeRef (pointer <> "/subject") (J.fiSubject fact)
    pred_ <- predicateRef (pointer <> "/predicate") (J.fiPredicate fact)
    val <- valueFromInput (pointer <> "/object") (J.fiObject fact)
    pure (ItemFact FactStmt {factSubject = subj, factPredicate = pred_, factValue = val, factSpan = Span pointer})

  consumesItems <-
    if null (J.ciConsumes input)
      then pure []
      else do
        entries <- forM (zip [0 ..] (J.ciConsumes input)) $ \(i, entry) -> do
          let pointer = idx "/consumes" i
          case entry of
            J.ConsumeStrongInput sr -> ConsumeStrong <$> strongRef pointer sr
            J.ConsumeFactInput fc -> do
              commit <- strongRef (pointer <> "/commit") (J.fciCommit fc)
              subj <- nodeRef (pointer <> "/subject") (J.fciSubject fc)
              checkIdent (pointer <> "/predicate") (J.fciPredicate fc)
              obj <- traverse (valueFromInput (pointer <> "/object")) (J.fciObject fc)
              pure
                ( ConsumeFact
                    FactConsume
                      { factConsumeCommit = commit
                      , factConsumeSubject = subj
                      , factConsumePredicate = J.fciPredicate fc
                      , factConsumeObject = obj
                      , factConsumeSpan = Span pointer
                      }
                )
        pure [ItemConsumes (ConsumesBlock entries (Span "/consumes"))]

  refsMap <- fmap Map.fromList $ forM (Map.toList (J.ciRefs input)) $ \(role, targets) -> do
    lowered <- forM (zip [0 ..] targets) $ \(i, target) -> strongRef (idx ("/refs/" <> role) i) target
    pure (role, lowered)

  pure
    CommitStmt
      { commitVerb = J.ciVerb input
      , commitItems = declareItems ++ factItems ++ consumesItems
      , commitRefs = refsMap
      , commitSpan = Span ""
      }

commitFromJson :: Text -> Either FromJsonError CommitStmt
commitFromJson json = case decodeInput json of
  Left err -> Left (FJDecodeError err)
  Right (input :: J.CommitInput) -> either (Left . FJInvalid) Right (commitStmtFromInput input)

-- ---------------------------------------------------------------------
-- MachineInput -> MachineStmt
-- ---------------------------------------------------------------------

-- | @kind@ is the one discriminant; @value@ is present for every variant
-- except @self@.
patternTermFromInput :: Text -> J.PatternTermInput -> Either JsonError PatternTerm
patternTermFromInput pointer input = case input of
  J.TermSelfInput -> Right TermSelf
  J.TermParamInput v -> checkIdent (pointer <> "/value") v >> Right (TermParam v)
  J.TermVarInput v -> checkIdent (pointer <> "/value") v >> Right (TermVar v)
  J.TermNodeInput v ->
    if isValidNodeRef v
      then Right (TermNode v)
      else invalid (pointer <> "/value") (dquote v <> " is not a valid node reference")

effectValueFromInput :: Text -> J.EffectValueInput -> Either JsonError EffectValue
effectValueFromInput pointer input = case input of
  J.EffectValueTermInput t -> EffectValueTerm <$> patternTermFromInput pointer t
  J.EffectValueStrInput v -> Right (EffectValueLiteral (LitString v))
  J.EffectValueNumberInput v -> Right (EffectValueLiteral (LitNumber v))
  J.EffectValueBooleanInput b -> Right (EffectValueLiteral (LitBoolean b))

-- | Shared by a guard's 'J.ExistsInput' hops and a chained retract's own
-- intermediate hops (jedelman/dmml#5) -- both are the same real shape,
-- a predicate plus a 'PatternTerm'.
patternHopsFromInput :: Text -> [J.PatternHopInput] -> Either JsonError [PatternHop]
patternHopsFromInput pointer hopInputs =
  forM (zip [0 ..] hopInputs) $ \(i, hop) -> do
    let hopPointer = idx pointer i
    checkIdent (hopPointer <> "/predicate") (J.phiPredicate hop)
    term <- patternTermFromInput (hopPointer <> "/term") (J.phiTerm hop)
    pure PatternHop {hopPredicate = J.phiPredicate hop, hopTerm = term}

existsExprFromInput :: Text -> J.ExistsInput -> Either JsonError ExistsExpr
existsExprFromInput pointer input = do
  anchor <- patternTermFromInput (pointer <> "/anchor") (J.eiAnchor input)
  when (null (J.eiHops input)) (invalid (pointer <> "/hops") "a pattern must have at least one hop")
  hops <- patternHopsFromInput (pointer <> "/hops") (J.eiHops input)
  pure ExistsExpr {existsPattern = Pattern anchor hops, existsSpan = Span pointer}

-- | Builds a 'MachineStmt' directly from a 'J.MachineInput'. Every
-- ident-shaped field (state names, transition names, params, guard hop
-- predicates, effect targets) is validated as a real DMML identifier
-- before being placed in the AST.
machineStmtFromInput :: J.MachineInput -> Either JsonError MachineStmt
machineStmtFromInput input = do
  node <- nodeRef "/node" (J.miNode input)

  states <- forM (zip [0 ..] (J.miStates input)) $ \(i, s) -> do
    let pointer = idx "/states" i
    checkIdent (pointer <> "/ident") (J.siIdent s)
    pure StateDecl {stateIdent = J.siIdent s, stateSpan = Span pointer}

  transitions <- forM (zip [0 ..] (J.miTransitions input)) $ \(i, t) -> do
    let pointer = idx "/transitions" i
    checkIdent (pointer <> "/ident") (J.tiIdent t)
    forM_ (zip [0 ..] (J.tiParams t)) $ \(pi_, p) -> checkIdent (idx (pointer <> "/params") pi_) p
    maybe (Right ()) (checkIdent (pointer <> "/from")) (J.tiFrom t)
    maybe (Right ()) (checkIdent (pointer <> "/to")) (J.tiTo t)

    guards <- forM (zip [0 ..] (J.tiGuards t)) $ \(gi, g) -> do
      let guardPointer = idx (pointer <> "/guards") gi
      ex <- existsExprFromInput (guardPointer <> "/exists") (J.giExists g)
      pure GuardClause {guardNegated = J.giNegated g, guardExists = ex, guardSpan = Span guardPointer}

    effects <- forM (zip [0 ..] (J.tiEffects t)) $ \(ei, e) -> do
      let effectPointer = idx (pointer <> "/effects") ei
      case e of
        J.EffectAssertInput ident ->
          checkIdent (effectPointer <> "/ident") ident
            >> Right (EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode ident)))
        J.EffectRetractInput ident ->
          checkIdent (effectPointer <> "/ident") ident
            >> Right (EffectRetract TermSelf [] (PredIdent "state") Nothing)
        J.EffectAssertGeneralInput subjIn predText valIn -> do
          subj <- patternTermFromInput (effectPointer <> "/subject") subjIn
          pred_ <- predicateRef (effectPointer <> "/predicate") predText
          val <- effectValueFromInput (effectPointer <> "/value") valIn
          pure (EffectAssert subj pred_ val)
        J.EffectRetractGeneralInput subjIn hopInputs predText mValIn -> do
          subj <- patternTermFromInput (effectPointer <> "/subject") subjIn
          hops <- patternHopsFromInput (effectPointer <> "/hops") hopInputs
          pred_ <- predicateRef (effectPointer <> "/predicate") predText
          mVal <- traverse (effectValueFromInput (effectPointer <> "/value")) mValIn
          pure (EffectRetract subj hops pred_ mVal)

    let hasContent = not (null guards) || (isJust (J.tiFrom t) && isJust (J.tiTo t)) || not (null effects)
    unless
      hasContent
      (invalid pointer "transition must have at least one of: a guard, a from+to pair, or an effect")

    pure
      TransitionDecl
        { transitionIdent = J.tiIdent t
        , transitionParams = J.tiParams t
        , transitionFrom = J.tiFrom t
        , transitionTo = J.tiTo t
        , transitionGuards = guards
        , transitionEffects = effects
        , transitionSpan = Span pointer
        }

  pure MachineStmt {machineNode = node, machineStates = states, machineTransitions = transitions, machineSpan = Span ""}

machineFromJson :: Text -> Either FromJsonError MachineStmt
machineFromJson json = case decodeInput json of
  Left err -> Left (FJDecodeError err)
  Right (input :: J.MachineInput) -> either (Left . FJInvalid) Right (machineStmtFromInput input)

-- ---------------------------------------------------------------------
-- ReferenceInput -> ReferenceStmt
-- ---------------------------------------------------------------------

referenceStmtFromInput :: J.ReferenceInput -> Either JsonError ReferenceStmt
referenceStmtFromInput input = do
  asName <- traverse (nodeRef "/asName") (J.riAsName input)
  target <- strongRef "/target" (J.riTarget input)
  pure ReferenceStmt {referenceTarget = target, referenceAsName = asName, referenceSpan = Span ""}

referenceFromJson :: Text -> Either FromJsonError ReferenceStmt
referenceFromJson json = case decodeInput json of
  Left err -> Left (FJDecodeError err)
  Right (input :: J.ReferenceInput) -> either (Left . FJInvalid) Right (referenceStmtFromInput input)

-- ---------------------------------------------------------------------
-- UpdateInput: batching many commits/machines into one authoring call
-- ---------------------------------------------------------------------

-- | Commits WITHIN one batch are simultaneous -- checked accordingly: a
-- duplicate (subject, predicate) across two commits in the same batch
-- is rejected the same way a self-collision within one commit already
-- is. Commits ACROSS separate batches are sequential -- a later batch's
-- commit legitimately overwriting an earlier batch's fact is correct
-- append-only-log behavior, so nothing is checked across batch
-- boundaries (a fresh @seen@ map per batch, below).
duplicateFactsAcrossBatch :: Int -> [J.CommitInput] -> [JsonError]
duplicateFactsAcrossBatch bi commits = snd (foldl' step (Map.empty, []) flat)
  where
    flat :: [(Int, Int, Text, Text)]
    flat = [(ci, fi, J.fiSubject f, J.fiPredicate f) | (ci, c) <- zip [0 ..] commits, (fi, f) <- zip [0 ..] (J.ciFacts c)]

    step :: (Map (Text, Text) (Int, Int), [JsonError]) -> (Int, Int, Text, Text) -> (Map (Text, Text) (Int, Int), [JsonError])
    step (seen, errs) (ci, fi, subj, pred_) =
      let key = (subj, pred_)
       in case Map.lookup key seen of
            Just (firstCi, firstFi) ->
              ( seen
              , errs
                  ++ [ JsonError
                        ("/update/" <> tshow bi <> "/commits/" <> tshow ci <> "/facts/" <> tshow fi)
                        ( "duplicate ("
                            <> subj
                            <> ", "
                            <> pred_
                            <> ") within the same batch -- already asserted at /update/"
                            <> tshow bi
                            <> "/commits/"
                            <> tshow firstCi
                            <> "/facts/"
                            <> tshow firstFi
                            <> "; commits in the same batch describe one simultaneous snapshot, so there is no legitimate later-wins here"
                        )
                     ]
              )
            Nothing -> (Map.insert key (ci, fi) seen, errs)

-- | Unlike every single-item @*FromJson@ function, this never fails
-- fast: a pile of facts is exactly the case where finding only the
-- first mistake and forcing another round trip is most costly, so every
-- commit and machine in every batch is built independently and every
-- failure is collected before returning.
updateFromJson :: Text -> Either UpdateFromJsonError Update
updateFromJson json = case decodeInput json of
  Left err -> Left (UFJDecodeError err)
  Right (input :: J.UpdateInput) ->
    let results = map processBatch (zip [0 ..] (J.uiUpdate input))
        allErrs = concatMap fst results
        batches = map snd results
     in if null allErrs then Right (Update batches) else Left (UFJInvalid allErrs)
  where
    processBatch :: (Int, J.BatchInput) -> ([JsonError], Batch)
    processBatch (bi, batchInput) =
      let commitResults = map commitStmtFromInput (J.biCommits batchInput)
          commitErrs =
            [ JsonError ("/update/" <> tshow bi <> "/commits/" <> tshow ci <> jePointer e) (jeMessage e)
            | (ci, Left e) <- zip [0 :: Int ..] commitResults
            ]
          commits = [s | Right s <- commitResults]

          machineResults = map machineStmtFromInput (J.biMachines batchInput)
          machineErrs =
            [ JsonError ("/update/" <> tshow bi <> "/machines/" <> tshow mi <> jePointer e) (jeMessage e)
            | (mi, Left e) <- zip [0 :: Int ..] machineResults
            ]
          machines = [s | Right s <- machineResults]

          dupErrs = duplicateFactsAcrossBatch bi (J.biCommits batchInput)
       in (commitErrs ++ machineErrs ++ dupErrs, Batch commits machines)

-- ---------------------------------------------------------------------
-- JSON decoding
-- ---------------------------------------------------------------------

decodeInput :: FromJSON a => Text -> Either String a
decodeInput = eitherDecode . BL.fromStrict . TE.encodeUtf8
