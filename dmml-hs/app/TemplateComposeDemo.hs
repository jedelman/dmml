{-# LANGUAGE OverloadedStrings #-}

-- | Real, no-LLM proof of SPEC.md §19.4 / DMML.TemplateBank
-- (@written-world#139@): materializes a real @.dmml@ world, then
-- selects and renders text purely by closed-catalog membership +
-- deterministic slot-fill -- zero model calls anywhere in this file.
--
-- Usage: template-compose-demo examples/template-compose-demo/world.dmml
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (Literal (LitString), NodeRef (NodeRef), Value (ValueLiteral))
import DMML.Materialize (WorldSnapshot, applyCommit, emptySnapshot)
import DMML.Surface (parseCommitSurface)
import DMML.TemplateBank (Template (..), eligibleTemplates, renderTemplate)

-- The catalog: closed, hand-vetted, curated content -- exactly the
-- "base sprite/decorator set" precedent, in prose form. Two templates
-- deliberately have MUTUALLY EXCLUSIVE coverage (corroded vs. pristine)
-- so this demo can prove the wrong one is never selected, not just
-- that the right one is -- and both are bound to `MetalObject`, so the
-- world's third entity (a `Spirit` sharing `condition = corroded`) is
-- the real test of type-binding specifically: it must never match
-- `worn-corroded`, even though the bare attribute tag alone would.
catalog :: [Template]
catalog =
  [ Template
      "worn-corroded"
      (NodeRef ["MetalObject"])
      [("condition", ValueLiteral (LitString "corroded"))]
      "{subject} looks worn, its surface corroded with age."
  , Template
      "gleaming-pristine"
      (NodeRef ["MetalObject"])
      [("condition", ValueLiteral (LitString "pristine"))]
      "{subject} gleams, freshly forged and untouched."
  , Template
      "metal-generic"
      (NodeRef ["MetalObject"])
      [("material", ValueLiteral (LitString "metal"))]
      "{subject} is built of metal."
  ]

main :: IO ()
main = do
  args <- getArgs
  case args of
    [worldPath] -> do
      src <- TIO.readFile worldPath
      case parseCommitSurface src of
        Left err -> putStrLn (worldPath <> ":\n" <> errorBundlePretty err) >> exitFailure
        Right stmt -> do
          let snap = applyCommit "world" emptySnapshot stmt
          describe snap "npc/watcher"
          describe snap "npc/keeper"
          describe snap "creature/wraith"
    _ -> putStrLn "usage: template-compose-demo <world.dmml>" >> exitFailure

describe :: WorldSnapshot -> T.Text -> IO ()
describe snap subject = do
  let eligible = eligibleTemplates snap subject catalog
  putStrLn ("=== " <> T.unpack subject <> " ===")
  putStrLn ("eligible templates: " <> show (map templateId eligible))
  mapM_ (\tpl -> TIO.putStrLn ("  -> " <> renderTemplate subject tpl)) eligible
  putStrLn ""
