{-# LANGUAGE OverloadedStrings #-}

-- | Real, no-LLM proof of SPEC.md §19.4 / DMML.TemplateBank
-- (@written-world#139@): materializes a real @.dmml@ world, then
-- selects and renders text purely by real guard evaluation +
-- deterministic slot-fill -- zero model calls anywhere in this file.
--
-- Each catalog template's eligibility condition is written as literal
-- DMML guard text, parsed by the real 'DMML.Surface.parseMachineSurface'
-- (embedded in a throwaway single-transition machine purely so the real
-- parser has something to parse -- the machine itself is never fired,
-- only its transition's own real, parsed 'DMML.Ast.GuardClause' list is
-- kept). "Bound to type" is not a special case: @guard self \`a\`
-- MetalObject@ is just another guard clause, same as @guard self
-- \`condition\` corroded@.
--
-- Usage: template-compose-demo examples/template-compose-demo/world.dmml
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (GuardClause, machineTransitions, transitionGuards)
import DMML.Materialize (WorldSnapshot, applyCommit, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import DMML.TemplateBank (Template (..), eligibleTemplates, renderTemplate)

-- | Parses one throwaway machine's single transition and returns its
-- real, parsed guard list -- never fired, never governs anything;
-- purely a vehicle for getting real DMML guard TEXT through the real
-- parser rather than hand-building 'GuardClause' values in Haskell.
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

-- The catalog: closed, hand-vetted, curated content -- exactly the
-- "base sprite/decorator set" precedent, in prose form. Each
-- eligibility condition is real DMML guard text. `worn-corroded` and
-- `gleaming-pristine` are mutually exclusive by their own attribute
-- values; `not-pristine-metal` proves negation composes through this
-- same path for free (matches anything metal that ISN'T pristine --
-- npc/watcher, not npc/keeper).
catalog :: [Template]
catalog =
  [ Template
      "worn-corroded"
      (guardsFromText "guard self `a` type/metalobject\nguard self `condition` state/corroded")
      "{subject} looks worn, its surface corroded with age."
  , Template
      "gleaming-pristine"
      (guardsFromText "guard self `a` type/metalobject\nguard self `condition` state/pristine")
      "{subject} gleams, freshly forged and untouched."
  , Template
      "metal-generic"
      (guardsFromText "guard self `a` type/metalobject\nguard self `material` stuff/metal")
      "{subject} is built of metal."
  , Template
      "not-pristine-metal"
      (guardsFromText "guard self `a` type/metalobject\nguard not self `condition` state/pristine")
      "{subject} has clearly seen use."
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
