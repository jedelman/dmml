{-# LANGUAGE OverloadedStrings #-}

-- | Oracle for the "informed authoring" checkpoint
-- (dmml/compliance-surface-informed/): same shape-check as
-- ComplianceCheckSurface.hs, PLUS a check that only makes sense once an
-- agent has been handed a world snapshot as context -- did it
-- needlessly redeclare a predicate the snapshot already showed as
-- declared? Not a parse error either way (redeclaring isn't invalid
-- DMML), but it's the one concrete, automatable signal for whether
-- being handed real state actually changed authoring behavior versus
-- the blind baseline every earlier checkpoint used.
--
-- The "already declared" set is fixed to examples/shrine-genesis.dmml's
-- own declares (accepts, witnessedBy, sealedBy, belongsTo, holds,
-- state) -- this checkpoint's scenarios are all authored against that
-- exact seed, not a general-purpose tool.
--
-- Reads newline-delimited JSON records from stdin:
--   {"id", "model", "scenario", "reply"}
-- Writes one newline-delimited JSON verdict per record to stdout:
--   {"id", "model", "scenario", "fenced", "outcome", "error", "redeclaresExisting": [...]}
module Main (main) where

import Data.Aeson
import Data.Aeson.KeyMap (KeyMap)
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString.Lazy.Char8 as BLC
import Data.Text (Text)
import qualified Data.Text as T
import System.IO (isEOF)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast
import qualified DMML.Ast
import DMML.Surface (parseCommitSurface)

alreadyDeclared :: [Text]
alreadyDeclared = ["accepts", "witnessedBy", "sealedBy", "belongsTo", "holds", "state"]

extractFence :: Text -> Maybe Text
extractFence s = do
  let (_, afterOpen0) = T.breakOn "```" s
  afterOpen1 <- T.stripPrefix "```" afterOpen0
  let bodyStart = case T.breakOn "\n" afterOpen1 of
        (_, rest) -> case T.stripPrefix "\n" rest of
          Just r -> r
          Nothing -> afterOpen1
  let (body, afterClose0) = T.breakOn "```" bodyStart
  _ <- T.stripPrefix "```" afterClose0
  let trimmed = T.strip body
  if T.null trimmed then Nothing else Just trimmed

data Verdict = Verdict
  { vFenced :: Bool
  , vOutcome :: Text
  , vError :: Maybe Text
  , vRedeclares :: [Text]
  , vFacts :: [Text]
  }

declaredIdents :: CommitStmt -> [Text]
declaredIdents stmt = [declareIdent d | ItemDeclare d <- commitItems stmt]

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

predText :: PredicateRef -> Text
predText RdfType = "a"
predText (PredIdent t) = t

valueText :: DMML.Ast.Value -> Text
valueText (ValueNode n) = nodeRefText n
valueText (ValueLiteral (LitString s)) = "\"" <> s <> "\""
valueText (ValueLiteral (LitNumber n)) = n
valueText (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

-- | Every fact as "subject.predicate=value" -- lets a human inspect
-- which predicate NAME a model actually chose for a relationship,
-- which is the real question this checkpoint asks (not just "did it
-- parse").
factSummaries :: CommitStmt -> [Text]
factSummaries stmt =
  [nodeRefText (factSubject f) <> "." <> predText (factPredicate f) <> "=" <> valueText (factValue f) | ItemFact f <- commitItems stmt]

checkReply :: Text -> Verdict
checkReply reply =
  case extractFence reply of
    Just fenced -> classify True fenced
    Nothing ->
      let trimmed = T.strip reply
       in if T.null trimmed then Verdict False "rejected" (Just "reply was empty") [] [] else classify False trimmed
  where
    classify fenced candidate = case parseCommitSurface candidate of
      Right stmt ->
        Verdict fenced "accepted" Nothing [d | d <- declaredIdents stmt, d `elem` alreadyDeclared] (factSummaries stmt)
      Left err -> Verdict fenced "rejected" (Just (T.pack (errorBundlePretty err))) [] []

main :: IO ()
main = loop
  where
    loop = do
      eof <- isEOF
      if eof
        then pure ()
        else do
          line <- getLine
          case decode (BLC.pack line) :: Maybe Object of
            Nothing -> loop
            Just obj -> do
              let getStr k = case KM.lookup k obj of
                    Just (String t) -> t
                    _ -> ""
                  idv = getStr "id"
                  model = getStr "model"
                  scenario = getStr "scenario"
                  reply = getStr "reply"
                  v = checkReply reply
                  out =
                    object
                      [ "id" .= idv
                      , "model" .= model
                      , "scenario" .= scenario
                      , "fenced" .= vFenced v
                      , "outcome" .= vOutcome v
                      , "error" .= vError v
                      , "redeclaresExisting" .= vRedeclares v
                      , "facts" .= vFacts v
                      ]
              BLC.putStrLn (encode out)
              loop
