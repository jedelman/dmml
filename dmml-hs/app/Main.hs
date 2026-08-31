{-# LANGUAGE OverloadedStrings #-}

-- | The Haskell mirror of @dmml/examples/agent_authoring_demo.rs@ -- the
-- same three cases, run against this translation instead of the real
-- Rust crate, to check the translation actually agrees with the real
-- compiled Rust binary rather than just "looking right."
module Main (main) where

import qualified Data.Aeson as Aeson
import qualified Data.ByteString.Lazy as BL
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO

import DMML.FromJson
import qualified DMML.Ast as Ast
import qualified DMML.Json as J

decodeText :: Aeson.FromJSON a => Text -> Either String a
decodeText = Aeson.eitherDecode . BL.fromStrict . TE.encodeUtf8

main :: IO ()
main = do
  TIO.putStrLn "=== 1. GRAMMAR.md's own documented commit shape, fed to the real parser ===\n"
  let grammarDocShaped =
        T.unlines
          [ "{"
          , "  \"update\": [{"
          , "    \"commits\": [{"
          , "      \"consumes\": [],"
          , "      \"produces\": \"_:claim1 <https://written-world.example/predicate/claim> \\\"the crisis consists in the old dying and the new not yet born\\\" .\\n\","
          , "      \"predicate\": \"asserts\","
          , "      \"created_at\": \"2026-08-31T00:00:00Z\""
          , "    }]"
          , "  }]"
          , "}"
          ]
  case decodeText grammarDocShaped :: Either String J.UpdateInput of
    Right _ -> TIO.putStrLn "PARSED (unexpected)"
    Left err -> TIO.putStrLn ("REJECTED at the JSON-decode step, before any DMML-level validation:\n  " <> T.pack err <> "\n")

  TIO.putStrLn "=== 2. A minimal, valid commit against the REAL CommitInput shape ===\n"
  let realShapeValid =
        T.unlines
          [ "{"
          , "  \"update\": [{"
          , "    \"commits\": [{"
          , "      \"verb\": \"asserts\","
          , "      \"declares\": [{\"kind\": \"relation\", \"name\": \"claim\"}],"
          , "      \"facts\": [{"
          , "        \"subject\": \"notebooks/interregnum\","
          , "        \"predicate\": \"claim\","
          , "        \"object\": {\"kind\": \"str\", \"value\": \"the crisis consists in the old dying and the new not yet born\"}"
          , "      }],"
          , "      \"consumes\": [],"
          , "      \"refs\": {}"
          , "    }]"
          , "  }]"
          , "}"
          ]
  case updateFromJson realShapeValid of
    Right upd -> TIO.putStrLn ("ACCEPTED: " <> T.pack (show (length (Ast.updateBatches upd))) <> " batch(es) built.\n")
    Left err -> TIO.putStrLn ("REJECTED (unexpected): " <> T.pack (show err) <> "\n")

  TIO.putStrLn "=== 3. The real documented agent mistake: a fact split across sibling commits in one batch ===\n"
  TIO.putStrLn
    "(from_json.rs's own UpdateInput doc comment: 'a model split `player/1 holds\nkey/1` into one commit and `player/1 holds key/2` into a sibling commit in\nthe same batch, silently dropping key/1 at materialization' -- reconstructed\nhere against this Haskell translation, not just quoted from the comment.)\n"
  let splitFactBatch =
        T.unlines
          [ "{"
          , "  \"update\": [{"
          , "    \"commits\": ["
          , "      {"
          , "        \"verb\": \"asserts\","
          , "        \"declares\": [{\"kind\": \"relation\", \"name\": \"holds\"}],"
          , "        \"facts\": [{\"subject\": \"player/1\", \"predicate\": \"holds\", \"object\": {\"kind\": \"node\", \"value\": \"key/1\"}}],"
          , "        \"consumes\": [],"
          , "        \"refs\": {}"
          , "      },"
          , "      {"
          , "        \"verb\": \"asserts\","
          , "        \"declares\": [{\"kind\": \"relation\", \"name\": \"holds\"}],"
          , "        \"facts\": [{\"subject\": \"player/1\", \"predicate\": \"holds\", \"object\": {\"kind\": \"node\", \"value\": \"key/2\"}}],"
          , "        \"consumes\": [],"
          , "        \"refs\": {}"
          , "      }"
          , "    ]"
          , "  }]"
          , "}"
          ]
  case updateFromJson splitFactBatch of
    Right _ -> TIO.putStrLn "ACCEPTED (would silently drop key/1 at materialization if this validator didn't exist)"
    Left err -> TIO.putStrLn ("REJECTED by the real validator:\n  " <> T.pack (show err))
