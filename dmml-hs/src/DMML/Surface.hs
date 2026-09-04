{-# LANGUAGE OverloadedStrings #-}

-- | A new text authoring surface for DMML commits (spike, 2026-08-31) —
-- see @SURFACE.md@ for the grammar and design rationale. Not the
-- retired text grammar; a new design, parsed straight into the same
-- 'DMML.Ast.CommitStmt' the JSON front-end ("DMML.FromJson") builds, so
-- every existing consumer of that type works unchanged regardless of
-- which front-end produced it.
module DMML.Surface
  ( parseCommitSurface
  , commitSurfaceParser
  , parseMachineSurface
  , machineSurfaceParser
  ) where

import Control.Monad (void)
import Data.Char (isAsciiLower, isAsciiUpper, isDigit)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import Data.Void (Void)
import Text.Megaparsec
import Text.Megaparsec.Char
import qualified Text.Megaparsec.Char.Lexer as L

import DMML.Ast
import DMML.FromJson (isValidIdent, isValidNodeRef)

type Parser = Parsec Void Text

-- | Space consumer that also crosses blank lines and full-line comments
-- (none defined yet, kept for symmetry with a typical @megaparsec@
-- lexer) — used between top-level constructs, per 'L.indentBlock's own
-- convention of taking a "consumes newlines too" consumer.
scn :: Parser ()
scn = L.space space1 empty empty

-- | Space consumer that stops at a newline — used within one logical
-- line, so indentation-sensitive block boundaries stay visible to
-- 'L.indentBlock'.
sc :: Parser ()
sc = L.space (void (some (char ' ' <|> char '\t'))) empty empty

lexeme :: Parser a -> Parser a
lexeme = L.lexeme sc

symbol :: Text -> Parser Text
symbol = L.symbol sc

spanHere :: Parser Span
spanHere = Span . T.pack . sourcePosPretty <$> getSourcePos

-- ---------------------------------------------------------------------
-- Lexical tokens — deliberately mirroring DMML.FromJson's own character
-- classes (isValidIdent/isValidNodeRef), reused as a post-hoc guard
-- below so the two front-ends can't silently drift into accepting
-- different shapes.
-- ---------------------------------------------------------------------

isIdentStart :: Char -> Bool
isIdentStart c = isAsciiLower c || isAsciiUpper c

isIdentChar :: Char -> Bool
isIdentChar c = isIdentStart c || isDigit c || c == '_'

pIdentRaw :: Parser Text
pIdentRaw = do
  c0 <- satisfy isIdentStart
  rest <- takeWhileP Nothing isIdentChar
  pure (T.cons c0 rest)

pIdent :: Parser Text
pIdent = lexeme $ do
  t <- pIdentRaw
  if isValidIdent t then pure t else fail (show t <> " is not a valid identifier")

isSegPieceChar :: Char -> Bool
isSegPieceChar c = isIdentChar c || c == '.'

pNodeRefRaw :: Parser Text
pNodeRefRaw = do
  c0 <- satisfy (\c -> isIdentStart c || isDigit c)
  rest <- takeWhileP Nothing (\c -> isSegPieceChar c || c == '/')
  pure (T.cons c0 rest)

-- | The validated raw text of a node reference, before it's split into
-- 'NodeRef' segments -- shared by 'pNodeRefTok' and 'pPatternTerm'
-- (whose @Node@ case, per 'DMML.Ast.PatternTerm', carries the raw text
-- form, not a structured 'NodeRef').
pNodeRefText :: Parser Text
pNodeRefText = lexeme $ do
  t <- pNodeRefRaw
  if isValidNodeRef t
    then pure t
    else fail (show t <> " is not a valid node reference")

pNodeRefTok :: Parser NodeRef
pNodeRefTok = NodeRef . T.splitOn "/" <$> pNodeRefText

pStringLit :: Parser Text
pStringLit = lexeme $ do
  _ <- char '"'
  t <- takeWhileP Nothing (/= '"')
  _ <- char '"'
  pure t

pBoolLit :: Parser Bool
pBoolLit =
  lexeme $
    (True <$ (string "true" <* notFollowedBy (satisfy isIdentChar)))
      <|> (False <$ (string "false" <* notFollowedBy (satisfy isIdentChar)))

-- | A bare numeral, per SURFACE.md's one-token-lookahead rule (mirrors
-- @from_json.rs@'s own note on the same ambiguity): a number NOT
-- immediately followed by @.@ or @/@ (which would make it the start of
-- a node-ref segment instead).
pNumberLit :: Parser Text
pNumberLit = lexeme . try $ do
  sign <- maybe "" T.singleton <$> optional (char '-')
  intPart <- takeWhile1P Nothing isDigit
  fracPart <- option "" (T.cons <$> char '.' <*> takeWhile1P Nothing isDigit)
  notFollowedBy (satisfy (\c -> c == '.' || c == '/' || isIdentChar c))
  pure (sign <> intPart <> fracPart)

pValue :: Parser Value
pValue =
  (ValueLiteral . LitString <$> pStringLit)
    <|> (ValueLiteral . LitBoolean <$> try pBoolLit)
    <|> (ValueLiteral . LitNumber <$> try pNumberLit)
    <|> (ValueNode <$> pNodeRefTok)

-- ---------------------------------------------------------------------
-- Fact-shaped lines
-- ---------------------------------------------------------------------

-- | @subject :: a Type@ — sugar for the rdf:type fact, same reading as
-- Haskell's own @::@ (type-of).
pTypeOf :: Parser CommitItem
pTypeOf = try $ do
  sp <- spanHere
  subj <- pNodeRefTok
  _ <- symbol "::"
  _ <- symbol "a"
  ty <- pNodeRefTok
  pure (ItemFact FactStmt {factSubject = subj, factPredicate = RdfType, factValue = ValueNode ty, factSpan = sp})

-- | @subject \`predicate\` value@ — infix backtick application.
pInfixFact :: Parser CommitItem
pInfixFact = try $ do
  sp <- spanHere
  subj <- pNodeRefTok
  _ <- symbol "`"
  pred_ <- pIdentRaw <* sc
  _ <- symbol "`"
  val <- pValue
  pure (ItemFact FactStmt {factSubject = subj, factPredicate = PredIdent pred_, factValue = val, factSpan = sp})

-- | @subject . predicate = value@ — dot-field-assignment form of the
-- same fact shape as 'pInfixFact', purely a style choice.
pDotFact :: Parser CommitItem
pDotFact = try $ do
  sp <- spanHere
  subj <- pNodeRefTok
  _ <- symbol "."
  pred_ <- pIdent
  _ <- symbol "="
  val <- pValue
  pure (ItemFact FactStmt {factSubject = subj, factPredicate = PredIdent pred_, factValue = val, factSpan = sp})

pDeclare :: Parser CommitItem
pDeclare = try $ do
  sp <- spanHere
  _ <- symbol "declare"
  kind <- (DeclRelation <$ symbol "relation") <|> (DeclAttribute <$ symbol "attribute")
  name <- pIdent
  pure (ItemDeclare DeclareStmt {declareKind = kind, declareIdent = name, declareSpan = sp})

-- ---------------------------------------------------------------------
-- consumes / refs blocks
-- ---------------------------------------------------------------------

pStrongRefTail :: Parser StrongRef
pStrongRefTail = do
  sp <- spanHere
  uri <- lexeme (takeWhile1P Nothing (\c -> c /= ' ' && c /= '\t' && c /= '\n' && c /= '#'))
  _ <- symbol "#"
  cid <- lexeme (takeWhile1P Nothing (\c -> c /= ' ' && c /= '\t' && c /= '\n'))
  pure StrongRef {strongRefUri = uri, strongRefCid = cid, strongRefSpan = sp}

-- | @subject . predicate@ or @subject . predicate = value@ — the
-- fact-consume's own subject/predicate/[object] line, reusing the same
-- dot syntax as an ordinary fact.
pFactConsumeTarget :: Parser (NodeRef, Text, Maybe Value)
pFactConsumeTarget = do
  subj <- pNodeRefTok
  _ <- symbol "."
  pred_ <- pIdent
  mval <- optional (symbol "=" *> pValue)
  pure (subj, pred_, mval)

data ConsumeItem = CStrong StrongRef | CFact StrongRef (NodeRef, Text, Maybe Value)

pConsumeEntry :: Parser (L.IndentOpt Parser ConsumeItem (NodeRef, Text, Maybe Value))
pConsumeEntry = do
  kw <- symbol "strong" <|> symbol "fact"
  ref <- pStrongRefTail
  case kw of
    "strong" -> pure (L.IndentNone (CStrong ref))
    _ -> pure (L.IndentSome Nothing (\tgts -> pure (CFact ref (head tgts))) pFactConsumeTarget)

pConsumesBlock :: Parser CommitItem
pConsumesBlock = do
  sp <- spanHere
  items <- L.indentBlock scn $ do
    _ <- symbol "consumes"
    pure (L.IndentSome Nothing pure (L.indentBlock scn (pConsumeEntry)))
  entries <- traverse toEntry items
  pure (ItemConsumes (ConsumesBlock {consumesEntries = entries, consumesSpan = sp}))
  where
    toEntry (CStrong ref) = pure (ConsumeStrong ref)
    toEntry (CFact ref (subj, pred_, mval)) =
      pure
        ( ConsumeFact
            FactConsume
              { factConsumeCommit = ref
              , factConsumeSubject = subj
              , factConsumePredicate = pred_
              , factConsumeObject = mval
              , factConsumeSpan = strongRefSpan ref
              }
        )

pRefLine :: Parser (Text, StrongRef)
pRefLine = do
  role <- pIdent
  ref <- pStrongRefTail
  pure (role, ref)

pRefsBlock :: Parser (Map Text [StrongRef])
pRefsBlock = do
  pairs <- L.indentBlock scn $ do
    _ <- symbol "refs"
    pure (L.IndentSome Nothing pure pRefLine)
  pure (Map.fromListWith (++) [(role, [ref]) | (role, ref) <- pairs])

-- ---------------------------------------------------------------------
-- machine
-- ---------------------------------------------------------------------

pKeyword :: Text -> Parser ()
pKeyword kw = lexeme (void (string kw <* notFollowedBy (satisfy isIdentChar)))

-- | @self@, @$param@, a multi-segment node reference (@key\/7@), or a
-- bare identifier read as a pattern variable. A single-segment bareword
-- is lexically identical whether meant as a literal one-word node or a
-- pattern variable -- this surface can't tell them apart, so it always
-- reads a slash-free bareword as a variable. A real, named limitation
-- (see SURFACE.md), not a silent wrong answer: write a real multi-
-- segment node reference if a literal node is what's meant.
pPatternTerm :: Parser PatternTerm
pPatternTerm =
  (TermSelf <$ try (pKeyword "self"))
    <|> (TermParam <$> try (char '$' *> pIdentRaw <* sc))
    <|> try (do
          t <- pNodeRefText
          if T.any (== '/') t then pure (TermNode t) else fail "not a multi-segment node reference"
        )
    <|> (TermVar <$> pIdent)

-- | @anchor \`predicate\` term (\`predicate\` term)*@ -- the same
-- infix-backtick idiom as an ordinary fact's predicate application,
-- reused here for a guard's hop chain. At least one hop, per
-- 'DMML.FromJson.existsExprFromInput's own "a pattern must have at
-- least one hop" rule.
pPattern :: Parser Pattern
pPattern = do
  anchor <- pPatternTerm
  hops <- some $ do
    _ <- symbol "`"
    p <- pIdentRaw <* sc
    _ <- symbol "`"
    t <- pPatternTerm
    pure PatternHop {hopPredicate = p, hopTerm = t}
  pure (Pattern anchor hops)

pGuardLine :: Parser GuardClause
pGuardLine = do
  sp <- spanHere
  _ <- symbol "guard"
  neg <- option False (True <$ symbol "not")
  pat <- pPattern
  pure GuardClause {guardNegated = neg, guardExists = ExistsExpr {existsPattern = pat, existsSpan = sp}, guardSpan = sp}

-- | An effect's value position: @self@\/@$param@ (node-valued, resolved
-- at fire time), a literal (string\/number\/bool), or a bare\/multi-
-- segment node reference read as a literal node -- deliberately NOT
-- reusing 'pPatternTerm' whole, since its @TermVar@ fallback (an
-- existential pattern variable) makes no sense in effect-value
-- position: nothing an effect asserts is existentially open, it's
-- either bound to context or a literal author wrote down.
pEffectValue :: Parser EffectValue
pEffectValue =
  (EffectValueTerm TermSelf <$ try (pKeyword "self"))
    <|> (EffectValueTerm . TermParam <$> try (char '$' *> pIdentRaw <* sc))
    <|> (EffectValueLiteral . LitString <$> pStringLit)
    <|> (EffectValueLiteral . LitBoolean <$> try pBoolLit)
    <|> (EffectValueLiteral . LitNumber <$> try pNumberLit)
    <|> (EffectValueTerm . TermNode <$> pNodeRefText)

-- | General form, generalized 2026-09-03: @assert <term> \`<predicate>\`
-- <value>@ \/ @retract <term> (\`<predicate>\` <term>)* \`<predicate>\`
-- [<value>]@ -- the same infix-backtick fact idiom used everywhere else
-- in this grammar, so an effect reads exactly like the fact it will
-- become once it fires. Falls back to the old bare @assert <ident>@\/
-- @retract <ident>@ sugar (always implicitly @self . state@) when the
-- general form's required backtick isn't there -- real, already-
-- committed machine examples use the old form, kept working rather
-- than force-migrated.
--
-- Retract's trailing value is OPTIONAL, added 2026-09-04: a real eval
-- (dev-journal/2026-09-04-complex-machine-eval.md) found three
-- independent free models unprompted writing a value there anyway, by
-- analogy with assert -- accepted rather than fought, per Jason's call.
-- See 'DMML.Ast.Effect'\'s own doc comment for what it does (and
-- doesn't yet) mean once resolved.
--
-- Retract's intermediate hops (also 2026-09-04, jedelman/dmml#5) let it
-- mirror a guard's own multi-hop 'DMML.Ast.Pattern' -- @retract self
-- \`witnessedBy\` self \`at\` $eruption@ is real output a free model
-- wrote unprompted. Parsed unambiguously via one real trick: after each
-- @\`predicate\`@, try to read ANOTHER @term \`predicate\`@ pair
-- (an intermediate hop always has a real term AND is always followed by
-- another backtick-predicate, since only the terminal position can end
-- the line); if that fails, this predicate IS the terminal one, and
-- whatever comes next (or nothing at all) is its optional value via
-- 'pEffectValue' -- the same superset-of-'PatternTerm' parser
-- 'pAssertGeneral' already uses, so only the terminal position can ever
-- be a literal, matching 'DMML.Ast.Effect'\'s own doc comment.
pEffectLine :: Parser Effect
pEffectLine =
  (symbol "assert" *> (try pAssertGeneral <|> pAssertStateSugar))
    <|> (symbol "retract" *> (try pRetractGeneral <|> pRetractStateSugar))
  where
    pAssertGeneral = do
      subj <- pPatternTerm
      _ <- symbol "`"
      p <- pIdentRaw <* sc
      _ <- symbol "`"
      val <- pEffectValue
      pure (EffectAssert subj (PredIdent p) val)
    pAssertStateSugar = do
      ident <- pIdent
      pure (EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode ident)))
    pBacktickIdent :: Parser Text
    pBacktickIdent = symbol "`" *> (pIdentRaw <* sc) <* symbol "`"
    pRetractGeneral = do
      subj <- pPatternTerm
      firstPred <- pBacktickIdent
      retractSteps subj [] firstPred
    retractSteps subj hopsAcc predName = do
      cont <- optional (try ((,) <$> pPatternTerm <*> pBacktickIdent))
      case cont of
        Just (term, nextPred) -> retractSteps subj (hopsAcc ++ [PatternHop {hopPredicate = predName, hopTerm = term}]) nextPred
        Nothing -> do
          mval <- optional (try pEffectValue)
          pure (EffectRetract subj hopsAcc (PredIdent predName) mval)
    pRetractStateSugar = do
      _ident <- pIdent
      pure (EffectRetract TermSelf [] (PredIdent "state") Nothing)

-- | @from -> to@ -- a transition's optional state-pair line.
pFromTo :: Parser (Text, Text)
pFromTo = try $ do
  from <- pIdent
  _ <- symbol "->"
  to <- pIdent
  pure (from, to)

data TransitionLine = TLFromTo (Text, Text) | TLGuard GuardClause | TLEffect Effect

pTransitionLine :: Parser TransitionLine
pTransitionLine =
  (TLFromTo <$> pFromTo)
    <|> (TLGuard <$> pGuardLine)
    <|> (TLEffect <$> pEffectLine)

pParamList :: Parser [Text]
pParamList = between (symbol "(") (symbol ")") (pIdent `sepBy` symbol ",")

-- | @transition ident(params, ...)@ header, then indented guard/from-to/
-- effect lines. Same "must have at least one of: a guard, a from+to
-- pair, or an effect" rule as the JSON front-end's
-- @machine_stmt_from_input@.
pTransitionBlock :: Parser TransitionDecl
pTransitionBlock = do
  sp <- spanHere
  (ident, params, ls) <- L.indentBlock scn $ do
    _ <- symbol "transition"
    ident <- pIdent
    params <- pParamList
    pure (L.IndentMany Nothing (\ls -> pure (ident, params, ls)) pTransitionLine)
  let fromTos = [ft | TLFromTo ft <- ls]
      guards = [g | TLGuard g <- ls]
      effects = [e | TLEffect e <- ls]
      (from, to) = case fromTos of
        (ft : _) -> (Just (fst ft), Just (snd ft))
        [] -> (Nothing, Nothing)
      hasContent = not (null guards) || (from /= Nothing && to /= Nothing) || not (null effects)
  if hasContent
    then
      pure
        TransitionDecl
          { transitionIdent = ident
          , transitionParams = params
          , transitionFrom = from
          , transitionTo = to
          , transitionGuards = guards
          , transitionEffects = effects
          , transitionSpan = sp
          }
    else fail "transition must have at least one of: a guard, a from+to pair, or an effect"

pStateLine :: Parser StateDecl
pStateLine = do
  sp <- spanHere
  i <- pIdent
  pure StateDecl {stateIdent = i, stateSpan = sp}

pStatesBlock :: Parser [StateDecl]
pStatesBlock = L.indentBlock scn $ do
  _ <- symbol "states"
  pure (L.IndentSome Nothing pure pStateLine)

data MachineLine = MLStates [StateDecl] | MLTransition TransitionDecl

pMachineLine :: Parser MachineLine
pMachineLine =
  (MLStates <$> pStatesBlock)
    <|> (MLTransition <$> pTransitionBlock)

machineSurfaceParser :: Parser MachineStmt
machineSurfaceParser = do
  sp <- spanHere
  (node, ls) <- L.nonIndented scn $
    L.indentBlock scn $ do
      _ <- symbol "machine"
      node <- pNodeRefTok
      pure (L.IndentSome Nothing (\ls -> pure (node, ls)) pMachineLine)
  let states = concat [s | MLStates s <- ls]
      transitions = [t | MLTransition t <- ls]
  pure MachineStmt {machineNode = node, machineStates = states, machineTransitions = transitions, machineSpan = sp}

parseMachineSurface :: Text -> Either (ParseErrorBundle Text Void) MachineStmt
parseMachineSurface = parse (machineSurfaceParser <* scn <* eof) "<surface>"

-- ---------------------------------------------------------------------
-- commit
-- ---------------------------------------------------------------------

data CommitLine = CLItem CommitItem | CLRefs (Map Text [StrongRef])

pCommitLine :: Parser CommitLine
pCommitLine =
  (CLItem <$> pDeclare)
    <|> (CLItem <$> pConsumesBlock)
    <|> (CLRefs <$> pRefsBlock)
    <|> (CLItem <$> pTypeOf)
    <|> (CLItem <$> pInfixFact)
    <|> (CLItem <$> pDotFact)

-- | A repeated (subject, predicate) pair within one commit's own facts
-- is never something an author means to do -- the second occurrence
-- would silently overwrite the first with no signal at all (the exact
-- rule 'DMML.FromJson.commitStmtFromInput's own duplicate check
-- enforces on the JSON side; this surface had no equivalent until a
-- hand-authored genesis file tripped over it for real). Checked here on
-- the built 'CommitItem' list, one pass, before the commit is accepted.
predRefText :: PredicateRef -> Text
predRefText RdfType = "a"
predRefText (PredIdent t) = t

checkNoDuplicateFacts :: [CommitItem] -> Parser ()
checkNoDuplicateFacts items = go Map.empty [f | ItemFact f <- items]
  where
    go :: Map (Text, Text) Int -> [FactStmt] -> Parser ()
    go _ [] = pure ()
    go seen (f : rest) =
      let key = (T.intercalate "/" (nodeRefSegments (factSubject f)), predRefText (factPredicate f))
       in case Map.lookup key seen of
            Just _ ->
              fail
                ( "duplicate ("
                    <> T.unpack (fst key)
                    <> ", "
                    <> T.unpack (snd key)
                    <> ") within this commit -- the second occurrence would silently overwrite the first"
                )
            Nothing -> go (Map.insert key (Map.size seen) seen) rest

commitSurfaceParser :: Parser CommitStmt
commitSurfaceParser = do
  sp <- spanHere
  (verb, ls) <- L.nonIndented scn $
    L.indentBlock scn $ do
      _ <- symbol "commit"
      verb <- pIdent
      pure (L.IndentSome Nothing (\ls -> pure (verb, ls)) pCommitLine)
  let items = [i | CLItem i <- ls]
      refs = Map.unionsWith (++) [m | CLRefs m <- ls]
  checkNoDuplicateFacts items
  pure CommitStmt {commitVerb = verb, commitItems = items, commitRefs = refs, commitSpan = sp}

parseCommitSurface :: Text -> Either (ParseErrorBundle Text Void) CommitStmt
parseCommitSurface = parse (commitSurfaceParser <* scn <* eof) "<surface>"
