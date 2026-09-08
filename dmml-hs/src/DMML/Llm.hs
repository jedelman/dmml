{-# LANGUAGE OverloadedStrings #-}

-- | A minimal BYOK (bring-your-own-key) chat-completion client, for the
-- in-product authoring agents @written-world@'s @CLAUDE.md@ names
-- ("DMML authoring", "world interpretation") -- the free-tier path
-- specifically: the player supplies their own OpenRouter API key
-- (unmetered, costs the product nothing directly), which selects
-- whichever model they want (@google/gemini-3.7-flash@, GLM, etc.)
-- without a separate integration per provider.
--
-- Goes through the same embedded\/upcalled JVM 'DMML.Jgit' already
-- needs, via 'DMML.Http' -- the same generic transport 'DMML.Atproto'
-- uses, not a second curl-shelling path. This closes the same real gap
-- @written-world@'s own @Demiurge.hs@ had disclosed for its tier-2
-- OpenRouter fallback (a @System.Process@ call to @curl@, which
-- doesn't exist on Android either).
module DMML.Llm
  ( LlmError (..)
  , chatComplete
  ) where

import Data.Aeson (Value, (.:), (.=))
import qualified Data.Aeson as Aeson
import qualified Data.Aeson.Types as AesonT (parseEither)
import qualified Data.ByteString.Lazy as BL
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Http (HttpError, postJson)
import DMML.Jni (JvmHandle)

data LlmError
  = LlmTransportError HttpError
  | LlmResponseNotJson BL.ByteString
  | LlmResponseMissingContent BL.ByteString
  deriving (Eq, Show)

-- | One OpenRouter chat completion, no streaming, no tool calls --
-- exactly what a single-shot "write me this DMML content" or "answer
-- this world-building question" authoring call needs. @apiKey@ is the
-- player's own BYOK key, read from @OPENROUTER_API_KEY@ by the caller
-- (same env var name the dev-tooling dispatch pipeline in @CLAUDE.md@
-- already uses -- this is deliberately the SAME key for now, per
-- Jason's own call when dogfooding this first: simpler to start, with
-- the option to split into a separate per-player key later once this
-- moves past one person's own dogfooding). @model@ is any OpenRouter
-- model id (e.g. @\"google/gemini-3.7-flash\"@). Returns the raw
-- assistant message content, untouched -- callers that expect
-- structured output (a DMML commit, JSON) validate\/parse it
-- themselves; this function only proves a real chat completion
-- happened.
chatComplete :: JvmHandle -> Text -> Text -> Text -> Text -> IO (Either LlmError Text)
chatComplete jvm apiKey model systemPrompt userPrompt = do
  let payload =
        Aeson.object
          [ "model" .= model
          , "messages"
              .= [ Aeson.object ["role" .= ("system" :: Text), "content" .= systemPrompt]
                 , Aeson.object ["role" .= ("user" :: Text), "content" .= userPrompt]
                 ]
          ]
      headers = [("Authorization", "Bearer " <> T.unpack apiKey)]
  result <- postJson jvm "https://openrouter.ai/api/v1/chat/completions" headers payload
  pure $ case result of
    Left err -> Left (LlmTransportError err)
    Right body -> case Aeson.decode body of
      Nothing -> Left (LlmResponseNotJson body)
      Just v -> case extractContent v of
        Just content -> Right content
        Nothing -> Left (LlmResponseMissingContent body)
  where
    extractContent :: Value -> Maybe Text
    extractContent v =
      case AesonT.parseEither
        ( Aeson.withObject "response" $ \o -> do
            choices <- o .: "choices"
            case choices of
              (choice0 : _) -> do
                message <- choice0 .: "message"
                message .: "content"
              [] -> fail "empty choices"
        )
        v of
        Right content -> Just content
        Left _ -> Nothing
