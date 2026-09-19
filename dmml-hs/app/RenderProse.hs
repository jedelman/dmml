{-# LANGUAGE OverloadedStrings #-}

-- | CLI: turn a materialized world into sentences.
--
-- The runtime half of what @check-prose-coverage@ measures. That tool
-- answers "which facts have no sentence"; this one produces the
-- sentences, through the identical path -- same catalog, same
-- 'DMML.TemplateBank.eligibleTemplates' (real 'DMML.Guard.evalGuards'),
-- same 'DMML.TemplateBank.renderTemplateWith'. Nothing here selects or
-- generates text; a template is eligible or it is not, and its text is
-- fixed. Two runs over the same world produce the same prose, which is
-- the whole point of the closed-set design.
--
-- Exists as its own binary because the Jev driver needs it once per
-- round over every subject, and the measured lesson from
-- @scan-candidates@ is that a per-subject subprocess re-parsing the
-- whole world is the thing that makes a loop slow. @--json@ renders
-- every subject in one pass.
--
-- Usage:
--   render-prose [--catalog <file>]... [--subject <node>]... [--json] <world.dmml>...
module Main (main) where

import qualified Data.Aeson as A
import qualified Data.Aeson.Key as AK
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy.Char8 as BL
import Data.List (nub, sort)
import qualified Data.Map.Strict as Map
import Data.Text ()
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Materialize (WorldSnapshot (..), applyCommits)
import DMML.Surface (parseCommitSurface)
import DMML.TemplateBank (displayNameOf, eligibleTemplates, parseCatalog, renderTemplateWith)

main :: IO ()
main = do
  args <- getArgs
  case parseArgs args [] [] False [] of
    Nothing -> usage >> exitFailure
    Just (catalogPaths, wanted, asJson, worldPaths)
      | null worldPaths -> usage >> exitFailure
      | otherwise -> do
          catalog <- concat <$> mapM loadCatalog catalogPaths
          snap <- loadWorlds worldPaths
          let subjects
                | null wanted = sort (nub (map fst (Map.keys (snapshotFacts snap))))
                | otherwise = wanted
              proseFor s = [renderTemplateWith snap s t | t <- eligibleTemplates snap s catalog]
              rows = [(s, displayNameOf snap s, proseFor s) | s <- subjects]
          if asJson
            then
              BL.putStrLn . A.encode . A.object $
                [AK.fromText s A..= ps | (s, _, ps) <- rows, not (null ps)]
            else mapM_ render rows
  where
    render (subj, display, ps)
      | null ps = TIO.putStrLn ("  " <> subj <> "  -- nothing sayable")
      | otherwise = do
          TIO.putStrLn (subj <> (if display == subj then "" else "  (" <> display <> ")"))
          mapM_ (\p -> TIO.putStrLn ("    " <> p)) ps

    usage =
      putStrLn
        "usage: render-prose [--catalog <file>]... [--subject <node>]... [--json] <world.dmml>..."

    parseArgs [] cats subs j ps = Just (reverse cats, reverse subs, j, reverse ps)
    parseArgs ("--catalog" : c : rest) cats subs j ps = parseArgs rest (c : cats) subs j ps
    parseArgs ("--subject" : s : rest) cats subs j ps = parseArgs rest cats (T.pack s : subs) j ps
    parseArgs ("--json" : rest) cats subs _ ps = parseArgs rest cats subs True ps
    parseArgs (a : _) _ _ _ _ | take 2 a == "--" = Nothing
    parseArgs (a : rest) cats subs j ps = parseArgs rest cats subs j (a : ps)

    loadCatalog path = do
      raw <- BS.readFile path
      case parseCatalog (TE.decodeUtf8 raw) of
        Left err -> putStrLn (path <> ": " <> T.unpack err) >> exitFailure
        Right ts -> pure ts

    loadWorlds paths = applyCommits "world" <$> mapM one paths
      where
        one path = do
          raw <- BS.readFile path
          case parseCommitSurface (TE.decodeUtf8 raw) of
            Right c -> pure c
            Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure
