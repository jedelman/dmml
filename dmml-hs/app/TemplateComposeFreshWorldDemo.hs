{-# LANGUAGE OverloadedStrings #-}

-- | Real test against a world authored FRESH, from scratch, with the
-- explicit discipline Jason called for after the E1 run: no narrative
-- string literals anywhere. Every value -- including what E1's corpus
-- stored as free-text `role`/`state` -- is a declared node reference
-- (`role/oresmith`, `state/active`, ...), so EVERY fact is guard-
-- walkable and there is no fallback to `{attr:...}` literal
-- interpolation anywhere in this file. Where E1's real content forced
-- `DMML.TemplateBank.renderTemplateWith` to treat descriptive
-- attributes as slot-fill CONTENT because they could never be guard
-- CONDITIONS, this world's same kind of content (role, state) is fully
-- structural: it can gate template selection AND be rendered, because
-- it was never prose to begin with.
--
-- Usage: template-compose-fresh-world-demo examples/template-compose-fresh-world-demo/world.dmml
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (GuardClause, machineTransitions, transitionGuards)
import DMML.Materialize (WorldSnapshot, applyCommit, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import DMML.TemplateBank (Template (..), eligibleTemplates, renderTemplateWith)

guardsFromText :: T.Text -> [GuardClause]
guardsFromText src = case parseMachineSurface wrapped of
  Left err -> error ("template guard text failed to parse:\n" <> errorBundlePretty err)
  Right m -> case machineTransitions m of
    (t : _) -> transitionGuards t
    [] -> error "template machine had no transition"
  where
    wrapped =
      "machine tmpl/scratch\n\n  states\n    unused\n\n  transition check()\n"
        <> T.unlines (map ("    " <>) (T.lines src))

-- Every guard here targets vocabulary this world itself declares --
-- `role`/`state` are gate conditions now, not just interpolation
-- sources, because nothing about them is a string literal. Dotted
-- `{attr:role.name}`/`{attr:role.epithet}` markers resolve THROUGH the
-- node a subject's own `role` fact points at, to a real name fact
-- asserted on that node -- proving a display name is just another
-- fact, and that the SAME referenced node can carry several,
-- differently-purposed ones (`name` for plain use, `epithet` for a
-- more formal register) for different templates to choose between.
catalog :: [Template]
catalog =
  [ Template
      "smith-at-work"
      (guardsFromText "guard self `a` type/smith\nguard self `state` state/active\nguard self `role` role/oresmith")
      "{subject} works the forge at {attr:worksAt.name}, {attr:role.epithet}."
  , Template
      "smith-in-training"
      (guardsFromText "guard self `a` type/smith\nguard self `state` state/training")
      "{subject} still learns the trade, apprenticed at {attr:worksAt.name} as an {attr:role.name}."
  , Template
      "herbalist-active"
      (guardsFromText "guard self `a` type/herbalist\nguard self `state` state/active")
      "{subject} tends {attr:worksAt.name} as {attr:role.name}."
  , Template
      "any-active-worker"
      (guardsFromText "guard self `state` state/active")
      "{subject} is active and at work."
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
          describe snap "npc/smith"
          describe snap "npc/apprentice"
          describe snap "npc/herbalist"
    _ -> putStrLn "usage: template-compose-fresh-world-demo <world.dmml>" >> exitFailure

describe :: WorldSnapshot -> T.Text -> IO ()
describe snap subject = do
  let eligible = eligibleTemplates snap subject catalog
  putStrLn ("=== " <> T.unpack subject <> " ===")
  putStrLn ("eligible templates: " <> show (map templateId eligible))
  mapM_ (\tpl -> TIO.putStrLn ("  -> " <> renderTemplateWith snap subject tpl)) eligible
  putStrLn ""
