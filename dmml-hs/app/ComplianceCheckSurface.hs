{-# LANGUAGE OverloadedStrings #-}

-- | Ground-truth oracle for the Surface-syntax authoring-compliance
-- checkpoint (dmml/compliance-surface/) -- the same discipline as the
-- JSON checkpoint's dmml/examples/compliance_check.rs (run replies
-- through the REAL parser, not an approximation), aimed at
-- DMML.Surface.parseCommitSurface/parseMachineSurface instead of
-- from_json::update_from_json.
--
-- Reads newline-delimited JSON records from stdin:
--   {"id": "...", "model": "...", "scenario": "...", "reply": "<raw model text>", "kind": "commit"|"machine"}
-- ("kind" defaults to "commit" when absent, so records from before
-- machine authoring existed still work unchanged.)
-- Writes one newline-delimited JSON verdict per record to stdout:
--   {"id": "...", "model": "...", "scenario": "...", "fenced": true, "outcome": "accepted"|"rejected", "error": null|"..."}
--
-- Unlike the JSON oracle, there's no separate "unparseable JSON" vs
-- "invalid DMML content" split to report -- megaparsec either builds a
-- real CommitStmt/MachineStmt or it doesn't, one failure channel.
-- "fenced" still tracks whether a fenced code block was actually found,
-- same diagnostic value as it had there.
module Main (main) where

import Data.Aeson
import Data.Aeson.KeyMap (KeyMap)
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString.Lazy.Char8 as BLC
import Data.Text (Text)
import qualified Data.Text as T
import System.IO (isEOF)
import Text.Megaparsec (errorBundlePretty)

import DMML.Surface (parseCommitSurface, parseMachineSurface)

-- | First fenced code block in the reply, regardless of the language
-- tag on the opening fence (```dmml, ```haskell, plain ```, ...);
-- 'Nothing' if there's no fence at all. Mirrors from_json.rs's
-- extract_fenced_block's job for this different surface.
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
  }

checkReply :: Text -> Text -> Verdict
checkReply kind reply =
  case extractFence reply of
    Just fenced -> classify True fenced
    Nothing ->
      let trimmed = T.strip reply
       in if T.null trimmed then Verdict False "rejected" (Just "reply was empty") else classify False trimmed
  where
    classify fenced candidate = case kind of
      "machine" -> case parseMachineSurface candidate of
        Right _ -> Verdict fenced "accepted" Nothing
        Left err -> Verdict fenced "rejected" (Just (T.pack (errorBundlePretty err)))
      _ -> case parseCommitSurface candidate of
        Right _ -> Verdict fenced "accepted" Nothing
        Left err -> Verdict fenced "rejected" (Just (T.pack (errorBundlePretty err)))

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
                  kind = case KM.lookup "kind" obj of
                    Just (String t) -> t
                    _ -> "commit"
                  v = checkReply kind reply
                  out =
                    object
                      [ "id" .= idv
                      , "model" .= model
                      , "scenario" .= scenario
                      , "fenced" .= vFenced v
                      , "outcome" .= vOutcome v
                      , "error" .= vError v
                      ]
              BLC.putStrLn (encode out)
              loop
