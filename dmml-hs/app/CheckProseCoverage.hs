{-# LANGUAGE OverloadedStrings #-}

-- | CLI: which LIVE FACTS in a world have no prose, and which
-- predicates to write templates for first.
--
-- Distinct from @check-describable@, and the difference is the whole
-- reason this exists. That one asks, per SUBJECT, whether ANY
-- description mechanism is reachable -- a naming fact, or a governing
-- machine. It says so itself: it "does NOT check that today's specific
-- content already produced good prose." A world can pass it with every
-- subject named and not one relation sayable.
--
-- This asks the other question, per FACT: is there a template that
-- would actually say this? Measured 2026-09-19 across @examples/@: 105
-- distinct predicates in use, 12 templates in existence, 6 predicates
-- guarded by any of them. Every other fact in every demo world renders
-- as its own bare triple -- @apprenticesUnder master\/iolo@ -- which is
-- not prose and was never prose; it read acceptably only because the
-- demo vocabulary happened to be English verbs in the right form.
--
-- COVERAGE, precisely: a fact @(subject, predicate)@ is covered iff
-- some template ELIGIBLE for that subject is ABOUT that predicate --
-- either its guards test the predicate, or its text renders it through
-- an @{attr:...}@\/@{via:...}@ marker ('DMML.TemplateBank.
-- templatePredicates'). Eligibility is real 'DMML.Guard.evalGuards', so
-- a template that could never fire for this subject does not count as
-- covering anything about it.
--
-- Deliberately NOT a per-predicate table. A predicate is the wrong unit
-- -- @wardedBy@ wants different prose depending on whether the warden
-- is present, dead or bribed, which is exactly why 'DMML.TemplateBank'
-- keys on guards. The REPORT groups by predicate because that is how
-- the writing work is best ordered; what it checks is facts.
--
-- Usage:
--   check-prose-coverage [--catalog <file>]... [--ignore <pred>]... <world.dmml>...
--
-- Exits 0 when every live fact is covered, 1 otherwise. With no
-- catalog, everything is uncovered and the report is the full ranked
-- gap -- which is the useful thing to run first.
module Main (main) where

import qualified Data.ByteString as BS
import Data.List (nub, sortOn)
import qualified Data.Map.Strict as Map
import Data.Ord (Down (..))
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (Value (..))
import DMML.Materialize (WorldSnapshot (..), applyCommits, currentValue)
import DMML.Surface (parseCommitSurface)
import DMML.TemplateBank
  ( Template (..)
  , eligibleTemplates
  , parseCatalog
  , renderTemplateWith
  , templatePredicates
  )

-- | Predicates that are not things to SAY, in two kinds.
--
-- Machinery: @state@ is a machine's own lifecycle position and
-- @cleared@ is this corpus's reachability convention. Neither is a fact
-- about a place that anyone would read aloud, and counting them buries
-- the real gap under the two commonest facts in any world.
--
-- And the description mechanism itself: @name@, @epithet@ and
-- @description@ are what prose is MADE of, not subjects for it.
-- 'DMML.TemplateBank.displayNameOf' consumes @name@\/@epithet@, and a
-- @description@ fact is already finished prose. Added after the first
-- real run of this tool put @name@ (54 facts) and @description@ (30) at
-- the top of the gap list -- a ranking that would have sent someone off
-- to write "a template for the name predicate", which is a category
-- error the report itself induced.
--
-- @a@ is deliberately NOT here. "X is a Y" is real prose and a template
-- guarding @self \`a\` type\/...@ is the ordinary way to write it.
--
-- All overridable: another corpus may well want to describe its own
-- states, and should be able to say so.
defaultIgnored :: [Text]
defaultIgnored = ["state", "cleared", "name", "epithet", "description"]

main :: IO ()
main = do
  args <- getArgs
  case parseArgs args [] [] [] of
    Nothing -> usage >> exitFailure
    Just (catalogPaths, ignored, worldPaths)
      | null worldPaths -> usage >> exitFailure
      | otherwise -> do
          catalog <- concat <$> mapM loadCatalog catalogPaths
          snap <- loadWorlds worldPaths
          let ignore = if null ignored then defaultIgnored else ignored
              facts =
                [ (subj, p)
                | (subj, p) <- Map.keys (snapshotFacts snap)
                , p `notElem` ignore
                ]
              -- A fact's VALUE KIND decides how it can ever be covered,
              -- and the report is misleading without it.
              --
              -- 'DMML.Guard' structurally excludes literal-valued facts
              -- from a guard walk -- 'DMML.TemplateBank''s own haddock
              -- says so, faithfully to the real crate. So
              -- @guard self `year` ?y@ can NEVER hold for
              -- @weir\/old `year` 1740@, and a report that lists `year`
              -- next to `at` under "write a template for these" is
              -- asking for one thing that works and one that cannot.
              -- Found by writing templates for both and watching five
              -- predicates stay uncovered with a template each.
              --
              -- A literal is still coverable, just by the other route:
              -- a template made eligible by some NODE-valued fact on the
              -- same subject, rendering the literal through
              -- @{attr:<pred>}@ as CONTENT.
              isLiteral (subj, p) =
                case currentValue (subj, p) snap of
                  ((_, ValueLiteral _) : _) -> True
                  _ -> False
              coveredBy subj p =
                [ templateId t
                | t <- eligibleTemplates snap subj catalog
                , p `elem` templatePredicates t
                ]
              results = [((subj, p), coveredBy subj p) | (subj, p) <- facts]
              uncovered = [k | (k, []) <- results]
              covered = [k | (k, _ : _) <- results]
              -- A template whose {attr:...} path does not resolve is not
              -- caught anywhere else: 'renderTemplateWith' substitutes
              -- only the markers it CAN resolve and leaves the rest
              -- verbatim, so the marker text lands in the prose. A
              -- catalog is exactly where that happens -- write
              -- {attr:has.name} for a world whose objects carry no
              -- `name` fact and every sentence ships with a brace in it.
              -- Coverage that does not check this is not coverage.
              leaky =
                [ (subj, templateId t, rendered)
                | subj <- nub (map fst facts)
                , t <- eligibleTemplates snap subj catalog
                , let rendered = renderTemplateWith snap subj t
                , "{" `T.isInfixOf` rendered
                ]

          putStrLn $
            "check-prose-coverage: "
              <> show (length covered)
              <> " of "
              <> show (length facts)
              <> " live fact(s) covered by "
              <> show (length catalog)
              <> " template(s); "
              <> show (length ignore)
              <> " predicate(s) ignored as machinery ("
              <> T.unpack (T.intercalate ", " ignore)
              <> ")"

          if not (null leaky)
            then do
              putStrLn ""
              putStrLn "UNRESOLVED MARKERS -- these templates render a brace into their own prose:"
              mapM_
                ( \(subj, tid, r) ->
                    putStrLn ("  " <> T.unpack tid <> " on " <> T.unpack subj <> ":\n      " <> T.unpack r)
                )
                (take 10 leaky)
              putStrLn ""
              putStrLn (show (length leaky) <> " unresolved rendering(s). Fix these before counting coverage.")
              exitFailure
            else pure ()

          if null uncovered
            then putStrLn "every live fact has prose, and every eligible template renders clean."
            else do
              let rank ks =
                    sortOn (Down . length . snd) $
                      Map.toList (Map.fromListWith (++) [(p, [s]) | (s, p) <- ks])
                  nodeValued = rank [k | k <- uncovered, not (isLiteral k)]
                  litValued = rank [k | k <- uncovered, isLiteral k]
                  section title hint rows
                    | null rows = pure ()
                    | otherwise = do
                        putStrLn ""
                        putStrLn title
                        putStrLn "   facts  predicate              e.g."
                        mapM_
                          ( \(p, subjects) ->
                              putStrLn $
                                pad 8 (show (length subjects))
                                  <> pad 22 (T.unpack p)
                                  <> T.unpack (T.intercalate ", " (take 2 (nub subjects)))
                          )
                          rows
                        putStrLn ("   -> " <> hint)
              section
                "UNCOVERED, node-valued -- a guard on the predicate covers these:"
                "guard self `<pred>` ?x, then render {attr:<pred>}."
                nodeValued
              section
                "UNCOVERED, literal-valued -- NO guard can ever match these:"
                ( "DMML.Guard excludes literal-valued facts from a guard walk. Cover them with a "
                    <> "template made eligible by a node-valued fact on the same subject, rendering "
                    <> "{attr:<pred>} as content."
                )
                litValued
              putStrLn ""
              putStrLn $
                show (length nodeValued + length litValued)
                  <> " predicate(s) still render as a bare triple ("
                  <> show (length nodeValued)
                  <> " node-valued, "
                  <> show (length litValued)
                  <> " literal-valued)."
              exitFailure
  where
    pad n s = s <> replicate (max 1 (n - length s)) ' '

    usage =
      putStrLn
        "usage: check-prose-coverage [--catalog <file>]... [--ignore <pred>]... <world.dmml>..."

    parseArgs [] cats igns ps = Just (reverse cats, reverse igns, reverse ps)
    parseArgs ("--catalog" : c : rest) cats igns ps = parseArgs rest (c : cats) igns ps
    parseArgs ("--ignore" : i : rest) cats igns ps = parseArgs rest cats (T.pack i : igns) ps
    parseArgs (a : _) _ _ _ | take 2 a == "--" = Nothing
    parseArgs (a : rest) cats igns ps = parseArgs rest cats igns (a : ps)

    loadCatalog path = do
      raw <- BS.readFile path
      case parseCatalog (TE.decodeUtf8 raw) of
        Left err -> putStrLn (path <> ": " <> T.unpack err) >> exitFailure
        Right ts -> pure ts

    loadWorlds paths = do
      stmts <- mapM one paths
      pure (applyCommits "world" stmts)
      where
        one path = do
          raw <- BS.readFile path
          case parseCommitSurface (TE.decodeUtf8 raw) of
            Right c -> pure c
            Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure
