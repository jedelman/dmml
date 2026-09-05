{-# LANGUAGE OverloadedStrings #-}

-- | Real test against baked, messy, multi-agent-authored world content:
-- the E1 endurance run's real commit corpus (jedelman/dmml#1,
-- @compliance-endurance/results/commits@, 208 files, 4 real models'
-- output over 20 rounds) -- not a hand-crafted toy fixture. Proves
-- (and, honestly, disproves part of) SPEC.md §19.4 / DMML.TemplateBank
-- against real content for the first time.
--
-- Usage: template-compose-e1-demo <commits-dir>
module Main (main) where

import qualified Data.ByteString as BS
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import System.Directory (listDirectory)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt, GuardClause, machineTransitions, transitionGuards)
import DMML.Materialize (WorldSnapshot, applyCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import DMML.TemplateBank (Template (..), eligibleTemplates, renderTemplateWith)

-- Wraps real guard TEXT in a throwaway single-transition machine so the
-- real DMML.Surface parser (not hand-built AST) produces the real
-- DMML.Ast.GuardClause list -- same technique as template-compose-demo.
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

-- Real vocabulary, drawn from actually reading the corpus (`grep`, not
-- guessed): `:: a Miner`/`:: a Herbalist`, real `worksAt` relations.
-- `{attr:role}`/`{attr:purpose}` interpolate the subject's own real
-- literal-valued facts -- content, not eligibility.
catalog :: [Template]
catalog =
  [ Template
      "miner-at-work"
      (guardsFromText "guard self `a` Miner\nguard self `worksAt` mine/ninefathom")
      "{subject} works the ninefathom seam: {attr:role}."
  , Template
      "herbalist-of-oldroot"
      (guardsFromText "guard self `a` Herbalist\nguard self `worksAt` forest/oldroot")
      "{subject} tends oldroot: {attr:role}."
  ]

main :: IO ()
main = do
  args <- getArgs
  case args of
    [commitsDir] -> do
      names <- listDirectory commitsDir
      let paths = [commitsDir </> n | n <- names, T.isSuffixOf ".dmml" (T.pack n)]
      commitStmts <- fmap concat (mapM tryParseCommit paths)
      putStrLn ("parsed " <> show (length commitStmts) <> " real commit file(s) as ground truth\n")
      let snap = applyCommits "e1" commitStmts
      describe snap "npc/delver"
      describe snap "herbalist/onn"
      describe snap "npc/keeper"
      demonstrateLiteralGuardLimit
    _ -> putStrLn "usage: template-compose-e1-demo <commits-dir>" >> exitFailure

-- Real files include machine defs too (same corpus, same convention
-- every other tool here uses) -- try as a commit, silently skip if it
-- parses as a machine instead, hard-fail only on a genuine parse error
-- (neither shape), same discipline checkpoint-rebuild already uses.
tryParseCommit :: FilePath -> IO [CommitStmt]
tryParseCommit path = do
  raw <- BS.readFile path
  let src = TE.decodeUtf8 raw
  case parseCommitSurface src of
    Right stmt -> pure [stmt]
    Left commitErr -> case parseMachineSurface src of
      Right _ -> pure []
      Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure >> pure []

describe :: WorldSnapshot -> T.Text -> IO ()
describe snap subject = do
  let eligible = eligibleTemplates snap subject catalog
  putStrLn ("=== " <> T.unpack subject <> " ===")
  putStrLn ("eligible templates: " <> show (map templateId eligible))
  mapM_ (\tpl -> TIO.putStrLn ("  -> " <> renderTemplateWith snap subject tpl)) eligible
  putStrLn ""

-- The real, honest finding: a guard can't target a LITERAL-valued fact
-- at all -- DMML.Ast.PatternTerm has no literal case, only self/param/
-- multi-segment-node -- so attempting to write `guard self `role`
-- "first down the ninefathom shaft"` doesn't just fail to match, it
-- fails to PARSE. Demonstrated, not just cited from DMML.Guard's own
-- doc comment.
demonstrateLiteralGuardLimit :: IO ()
demonstrateLiteralGuardLimit = do
  putStrLn "=== attempting a guard against a literal-valued fact (role = \"...\") ==="
  case parseMachineSurface wrapped of
    Left err -> putStrLn ("REJECTED AT PARSE TIME (expected):\n" <> errorBundlePretty err)
    Right _ -> putStrLn "parsed (unexpected -- guards over literals should not parse)"
  where
    wrapped =
      "machine tmpl/scratch\n\n  states\n    unused\n\n  transition check()\n"
        <> "    guard self `role` \"first down the ninefathom shaft\"\n"
