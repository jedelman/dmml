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

pNodeRefTok :: Parser NodeRef
pNodeRefTok = lexeme $ do
  t <- pNodeRefRaw
  if isValidNodeRef t
    then pure (NodeRef (T.splitOn "/" t))
    else fail (show t <> " is not a valid node reference")

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
  pure CommitStmt {commitVerb = verb, commitItems = items, commitRefs = refs, commitSpan = sp}

parseCommitSurface :: Text -> Either (ParseErrorBundle Text Void) CommitStmt
parseCommitSurface = parse (commitSurfaceParser <* scn <* eof) "<surface>"
