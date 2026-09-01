{-# LANGUAGE OverloadedStrings #-}

-- | Parses SURFACE.md's worked example through the new text front-end
-- and checks the result against the same content authored as JSON
-- through the existing front-end -- one AST, two front-ends, checked to
-- actually agree rather than assumed to.
module Main (main) where

import Data.Text (Text)
import qualified Data.Text.IO as TIO
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast
import DMML.FromJson (commitFromJson)
import DMML.Surface (parseCommitSurface)

surfaceExample :: Text
surfaceExample =
  "commit mints\n\
  \  declare relation opensTo\n\
  \  declare attribute dampness\n\
  \\n\
  \  room/1 :: a Room\n\
  \  room/2 :: a Room\n\
  \  room/1 `opensTo` room/2\n\
  \  room/1 . dampness = 0.6\n"

equivalentJson :: Text
equivalentJson =
  "{\
  \  \"verb\": \"mints\",\
  \  \"declares\": [{\"kind\": \"relation\", \"name\": \"opensTo\"}, {\"kind\": \"attribute\", \"name\": \"dampness\"}],\
  \  \"facts\": [\
  \    {\"subject\": \"room/1\", \"predicate\": \"a\", \"object\": {\"kind\": \"node\", \"value\": \"Room\"}},\
  \    {\"subject\": \"room/2\", \"predicate\": \"a\", \"object\": {\"kind\": \"node\", \"value\": \"Room\"}},\
  \    {\"subject\": \"room/1\", \"predicate\": \"opensTo\", \"object\": {\"kind\": \"node\", \"value\": \"room/2\"}},\
  \    {\"subject\": \"room/1\", \"predicate\": \"dampness\", \"object\": {\"kind\": \"number\", \"value\": \"0.6\"}}\
  \  ],\
  \  \"consumes\": [],\
  \  \"refs\": {}\
  \}"

-- | Spans carry source position/JSON-pointer info that differs by
-- construction between the two front-ends -- irrelevant to whether the
-- two parses agree on WHAT was authored, so strip spans before
-- comparing (a real diff, not spans-inflated noise).
main :: IO ()
main = do
  putStrLn "=== Parsing the surface-syntax example ===\n"
  case parseCommitSurface surfaceExample of
    Left err -> putStrLn ("SURFACE PARSE FAILED:\n" <> errorBundlePretty err)
    Right surfaceStmt -> do
      putStrLn "Surface parse OK:"
      print surfaceStmt
      putStrLn "\n=== Parsing the equivalent JSON through the existing front-end ===\n"
      case commitFromJson equivalentJson of
        Left err -> putStrLn ("JSON PARSE FAILED (unexpected): " <> show err)
        Right jsonStmt -> do
          putStrLn "JSON parse OK:"
          print jsonStmt
          putStrLn "\n=== Comparing (verb, items, refs) ignoring spans ==="
          let eq =
                (commitVerb surfaceStmt == commitVerb jsonStmt)
                  && (map stripItemSpan (commitItems surfaceStmt) == map stripItemSpan (commitItems jsonStmt))
                  && (fmap (map stripRefSpan) (commitRefs surfaceStmt) == fmap (map stripRefSpan) (commitRefs jsonStmt))
          if eq
            then putStrLn "MATCH: both front-ends produced the same CommitStmt content."
            else putStrLn "MISMATCH: the two front-ends disagree -- see printed values above."
  where
    stripItemSpan (ItemDeclare d) = ItemDeclare d {declareSpan = Span ""}
    stripItemSpan (ItemFact f) = ItemFact f {factSpan = Span "", factSubject = stripNodeSpan (factSubject f)}
    stripItemSpan (ItemConsumes c) = ItemConsumes c {consumesSpan = Span ""}
    stripNodeSpan n = n
    stripRefSpan r = r {strongRefSpan = Span ""}
