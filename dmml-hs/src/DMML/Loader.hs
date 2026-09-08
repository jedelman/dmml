{-# LANGUAGE OverloadedStrings #-}

-- | Loads a real @commits/@-shaped directory of @.dmml@ files (fact-
-- commits and machine definitions, classified by which surface grammar
-- successfully parses) into a materialized 'WorldSnapshot' plus a
-- machine map. Extracted from written-world's own @cli/app/Main.hs@
-- (its @loadAll@\/@Loaded@\/@worldSnapshot@), where this exact logic
-- previously lived as @cli@-only application code, not shared library
-- code -- see @dmml@'s own
-- @dev-journal\/2026-09-07-android-canonical-structure-decisions.md@
-- for why that was a real gap, not a hypothetical one, and what moved
-- as a result. Every consumer of a real @commits\/@ directory (@cli@
-- itself, once it switches over -- not done in this change, it lives in
-- a sibling repo; 'DMML.JniBridge'\'s directory-based entry points,
-- added alongside this module) should call this, not reimplement it.
--
-- One deliberate behavior change from the code this was extracted
-- from: @cli@\'s original @loadOne@ called 'System.Exit.exitFailure'
-- directly on a parse error -- correct for a CLI's own @main@, wrong
-- for library code that can now be called from inside a JNI-loaded
-- @.so@ (exiting the process there would kill the whole Android app,
-- not just report an error). Every function here reports failure via
-- 'Either' instead.
--
-- Real ordering bug this logic's own history already found and fixed
-- (preserved from @cli@'s own doc comment, since the reasoning still
-- applies unchanged): a directory listing has NO ordering guarantee,
-- and a commit's own @consumes@ block only clears a fact asserted by
-- an EARLIER commit in the fold order -- so commits must load in a
-- real chronological proxy order, not filesystem readdir order and NOT
-- filename lexical order either (a fired commit's own generated
-- filename, @\<verb\>-\<timestampMs\>.dmml@, can sort before a hand-
-- authored @world.dmml@ it depends on -- \'a\' &lt; \'w\'). Sorted by
-- each file's real modification time instead -- correct because a
-- fired commit is always written strictly after whatever it consumes.
-- Residual, disclosed risk: the sort is stable, so a genuine mtime TIE
-- (coarse filesystem granularity, clock skew, two writes in the same
-- tick) falls back to readdir order -- narrowed, not eliminated.
module DMML.Loader
  ( Loaded (..)
  , loadDirectory
  , loadOne
  , identifiedCommits
  , machineMap
  , worldSnapshotFromDirectory
  ) where

import qualified Data.ByteString as BS
import Data.List (sortOn)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Directory (doesDirectoryExist, getModificationTime, listDirectory)
import System.FilePath ((</>))
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt, MachineStmt, NodeRef (..), machineNode, nodeRefSegments)
import DMML.LocalIdentity (localFileRef)
import DMML.Materialize (IdentifiedCommit (..), WorldSnapshot, applyIdentifiedCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | Every matched file's own bytes, path, and which of the two surface
-- grammars it parses as. A file that parses as NEITHER is a real,
-- reported error ('Left', from 'loadDirectory'), never silently
-- skipped.
data Loaded = Loaded
  { loadedPath :: FilePath
  , loadedBytes :: BS.ByteString
  , loadedCommit :: Maybe CommitStmt
  , loadedMachine :: Maybe MachineStmt
  }

-- | Loads every @*.dmml@ file directly inside @dir@ (non-recursive,
-- matching @cli@'s own convention), in real-modification-time order.
-- An absent directory loads as an empty list, not an error -- a caller
-- with nothing yet authored is a legitimate, common state (a fresh
-- Android install before its first sync, e.g.), not a failure. The
-- FIRST file that fails to parse as either grammar stops the load and
-- reports that file's error -- fails closed on the whole batch, same
-- discipline every other batch operation in this project already
-- follows, rather than silently dropping the one bad file and
-- materializing an incomplete world.
loadDirectory :: FilePath -> IO (Either String [Loaded])
loadDirectory dir = do
  exists <- doesDirectoryExist dir
  if not exists
    then pure (Right [])
    else do
      names <- listDirectory dir
      let paths = [dir </> n | n <- names, ".dmml" `isDmmlSuffix` n]
      timed <- mapM (\p -> (,) <$> getModificationTime p <*> pure p) paths
      goLoad (map snd (sortOn fst timed))
  where
    isDmmlSuffix suf s = suf == drop (length s - length suf) s
    goLoad [] = pure (Right [])
    goLoad (p : ps) = do
      loaded <- loadOne p
      case loaded of
        Left err -> pure (Left err)
        Right l -> fmap (fmap (l :)) (goLoad ps)

-- | Parses one file's bytes as either grammar, reporting the
-- commit-grammar's own parse error (arbitrarily, but consistently --
-- matches @cli@'s original choice) if BOTH fail.
loadOne :: FilePath -> IO (Either String Loaded)
loadOne path = do
  bytes <- BS.readFile path
  let src = TE.decodeUtf8 bytes
  pure $ case parseCommitSurface src of
    Right c -> Right (Loaded path bytes (Just c) Nothing)
    Left commitErr -> case parseMachineSurface src of
      Right m -> Right (Loaded path bytes Nothing (Just m))
      Left _ -> Left (path <> ":\n" <> errorBundlePretty commitErr)

-- | Every loaded fact-commit, with real per-file provenance
-- ('DMML.LocalIdentity.localFileRef', keyed on each file's own path --
-- so a retract can cite a real, distinct source per file, not one
-- shared label for the whole batch).
identifiedCommits :: [Loaded] -> [IdentifiedCommit]
identifiedCommits ls = [IdentifiedCommit (localFileRef (loadedPath l) (loadedBytes l)) c | l <- ls, Just c <- [loadedCommit l]]

-- | Every loaded machine, keyed by its own node -- genuinely multiple,
-- unlike 'DMML.JniBridge'\'s original single-machine functions (a real
-- v1 limit those functions' own header comment already disclosed).
-- Loading a whole directory removes that limit for free: every machine
-- file present gets loaded, not just one passed in separately.
machineMap :: [Loaded] -> Map.Map Text MachineStmt
machineMap ls = Map.fromList [(nodeRefText (machineNode m), m) | l <- ls, Just m <- [loadedMachine l]]

-- | The whole materialize-a-directory pipeline in one call: load every
-- file, then fold every fact-commit into a snapshot with real
-- per-file provenance, alongside every machine found.
worldSnapshotFromDirectory :: FilePath -> IO (Either String (WorldSnapshot, Map.Map Text MachineStmt))
worldSnapshotFromDirectory dir = do
  result <- loadDirectory dir
  pure $ case result of
    Left err -> Left err
    Right ls -> Right (applyIdentifiedCommits "world" (identifiedCommits ls), machineMap ls)
