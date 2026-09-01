{-# LANGUAGE OverloadedStrings #-}

-- | The real "trust step peer-to-peer needs that single-player didn't"
-- -- reworked 2026-09-02 per written-world's own "corruption as
-- content" principle (SPEC.md §12): a real divergence between two
-- branches is never silently resolved to whichever side is convenient,
-- and it's never blocked either -- it becomes real, structured content
-- in the world, the same way cross-repo drift became a `Drift` node
-- instead of a narrated guess or a rejected sync.
--
-- Given the set of commits I've authored since the last common point
-- with a peer, and the set they've authored since that same point: for
-- every (subject, predicate) both deltas touch, this MINTS two real
-- DMML files (not just a report) --
--   * a fact-commit recording the contest's existence, its disputed
--     subject/predicate, and every live option with its provenance
--   * a machine, states contested/resolved, with one transition
--     resolving it -- guarded by `self \`witnessedBy\` npc/keeper`,
--     reusing the exact vocabulary written-world/dmml-hs/examples/
--     shrine.dmml already established and hearth-genesis.dmml's own
--     keeper role text ("tends the hearth and admits petitioners to
--     the shrine") already names as this world's natural adjudicator
--     -- not a new authority invented for this.
--
-- Both mints are ordinary DMML, parsed and validated the same as any
-- other content before being written -- dogfooded, not just described.
--
-- Usage: check-divergence <mine-list-file> <peer-list-file> <output-dir> <mine-label> <peer-label>
--   Each list file: one .dmml path per line (may be empty).
-- Always exits 0 -- divergence is no longer a reason to block a merge,
-- only a reason to mint a contest. Prints which files (if any) were
-- written.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (Value (..), Literal (..), NodeRef (nodeRefSegments))
import DMML.Materialize (WorldSnapshot (..), applyCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

readListFile :: FilePath -> IO [FilePath]
readListFile p = do
  contents <- readFile p
  pure [l | l <- lines contents, not (null l)]

materializeFiles :: [FilePath] -> IO WorldSnapshot
materializeFiles paths = do
  srcs <- mapM TIO.readFile paths
  let parsed = zip paths (map parseCommitSurface srcs)
  case [(p, e) | (p, Left e) <- parsed] of
    ((p, e) : _) -> error (p <> ":\n" <> errorBundlePretty e)
    [] -> pure (applyCommits [stmt | (_, Right stmt) <- parsed])

-- | Node/segment-safe slug: node_ref segments only allow letters,
-- digits, underscore, and '.' as a piece separator -- no '/' (that's
-- the segment separator itself) and no '-'.
slug :: Text -> Text
slug = T.map (\c -> if c == '/' then '_' else c)

-- | A node-valued option is written as a bareword node reference, the
-- same shape any other node-typed value takes in this grammar
-- (`<node_ref> . <ident> = <value>`, where `<value>` can itself be a
-- bareword `<node_ref>`) -- no special-casing needed, `nodeRefSegments`
-- reassembled with '/' round-trips exactly what the fact's own object
-- already was. This used to be an unhandled `error` here ("cannot
-- embed a node-valued option in a contest record yet"), a real,
-- previously-flagged gap that crashed the whole process the first
-- time a real run actually hit it (two agents each pointing the same
-- predicate at a different node, not just a different string) --
-- exactly the class of divergence this primitive exists to surface,
-- not to crash on.
renderValueLiteral :: Value -> Text
renderValueLiteral (ValueNode n) = T.intercalate "/" (nodeRefSegments n)
renderValueLiteral (ValueLiteral (LitString s)) = "\"" <> s <> "\""
renderValueLiteral (ValueLiteral (LitNumber n)) = n
renderValueLiteral (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

mintContest :: Int -> (Text, Text) -> [(Text, Value)] -> (Text, Text)
mintContest n (subj, pred_) options = (factCommit, machine)
  where
    node = "contest/" <> slug subj <> "_" <> pred_ <> "_" <> T.pack (show n)
    optionLines =
      concat
        [ [ "  " <> node <> " . option" <> T.pack (show i) <> "Value = " <> renderValueLiteral v
          , "  " <> node <> " . option" <> T.pack (show i) <> "Source = \"" <> label <> "\""
          ]
        | (i, (label, v)) <- zip [1 :: Int ..] options
        ]
    factCommit =
      T.unlines $
        [ "commit raises"
        , "  declare relation disputes"
        , "  declare attribute subject"
        , "  declare attribute predicate"
        , "  declare attribute state"
        ]
          ++ ["  declare attribute option" <> T.pack (show i) <> "Value" | i <- [1 .. length options]]
          ++ ["  declare attribute option" <> T.pack (show i) <> "Source" | i <- [1 .. length options]]
          ++
          [ ""
          , "  " <> node <> " :: a Contest"
          , "  " <> node <> " . subject = \"" <> subj <> "\""
          , "  " <> node <> " . predicate = \"" <> pred_ <> "\""
          , "  " <> node <> " . state = \"contested\""
          ]
          ++ optionLines
          ++ ["  " <> node <> " `disputes` " <> subj]
    machine =
      T.unlines
        [ "machine " <> node
        , "  states"
        , "    contested"
        , "    resolved"
        , ""
        , "  transition resolve(witness)"
        , "    contested -> resolved"
        , "    guard self `witnessedBy` npc/keeper"
        , "    assert resolved"
        ]

-- | Every (subject, predicate) pair some already-materialized
-- @Contest@ node in this snapshot disputes -- resolved or not. Real gap
-- found designing (not yet running) a >2-peer full-mesh sync test: with
-- more than one peer, the SAME raw divergence can be re-discovered by
-- more than one pairwise check within a single sync round, minting a
-- second, redundant Contest for a pair an earlier check already
-- covered. First attempt at a fix here passed a THIRD "known" file list
-- (the invoking repo's own full corpus) and checked only that -- wrong,
-- caught by tracing a real hub-and-spoke sequence by hand before
-- running it: a repo that hasn't merged anyone yet has an empty known
-- set, so a still-uninformed receiver on either side of a merge would
-- still double-mint. The fix that's actually correct: check mineSnap
-- AND peerSnap themselves -- the exact two snapshots already being
-- compared for this merge -- since a Contest minted earlier and
-- already merged into EITHER side necessarily shows up as part of that
-- side's own "new since merge-base" files, no separate list needed.
alreadyContestedPairs :: WorldSnapshot -> Map.Map (Text, Text) ()
alreadyContestedPairs snap = Map.fromList [((s, p), ()) | (s, p) <- catPairs]
  where
    facts = snapshotFacts snap
    contestNodes = [subj | ((subj, p), ValueNode ty) <- Map.toList facts, p == "a", nodeRefSegments ty == ["Contest"]]
    getStr key = case Map.lookup key facts of Just (ValueLiteral (LitString s)) -> Just s; _ -> Nothing
    catPairs = [(s, p) | n <- contestNodes, Just s <- [getStr (n, "subject")], Just p <- [getStr (n, "predicate")]]

main :: IO ()
main = do
  args <- getArgs
  case args of
    [mineListPath, peerListPath, outDir, mineLabel, peerLabel] -> do
      mineFiles <- readListFile mineListPath
      peerFiles <- readListFile peerListPath
      mineSnap <- materializeFiles mineFiles
      peerSnap <- materializeFiles peerFiles
      let alreadyKnown = Map.union (alreadyContestedPairs mineSnap) (alreadyContestedPairs peerSnap)
      -- A shared (subject, predicate) key is only a real contest if the
      -- two independently-derived values actually differ -- two agents
      -- who never saw each other's work converging on the SAME value
      -- (both independently decide a mine is now "active") is agreement,
      -- not corruption, and minting a contest for it would itself be
      -- exactly the kind of false alarm "corruption as content" is
      -- supposed to avoid. Found for real, not hypothetically: an
      -- endurance run where 3 of 4 agents independently set the same
      -- freshly-dug mine to "active" got flagged as three-way disputed
      -- before this check existed.
      let overlapKeys =
            [ k
            | (k, mv) <- Map.toList (snapshotFacts mineSnap)
            , Just pv <- [Map.lookup k (snapshotFacts peerSnap)]
            , mv /= pv
            , not (k `Map.member` alreadyKnown)
            ]
      if null overlapKeys
        then putStrLn "no divergence"
        else
          mapM_
            ( \(n, key@(subj, pred_)) -> do
                let mv = snapshotFacts mineSnap Map.! key
                    pv = snapshotFacts peerSnap Map.! key
                    (factCommit, machine) = mintContest n key [(T.pack mineLabel, mv), (T.pack peerLabel, pv)]
                    factPath = outDir <> "/contest-" <> show n <> "-" <> T.unpack (slug subj) <> "-" <> T.unpack pred_ <> ".dmml"
                    machinePath = outDir <> "/contest-" <> show n <> "-" <> T.unpack (slug subj) <> "-" <> T.unpack pred_ <> ".machine.dmml"
                -- Verify what's about to be written is real, parseable DMML
                -- before committing to disk -- dogfooded, not just described.
                case parseCommitSurface factCommit of
                  Left err -> error ("minted contest fact-commit failed to parse (bug): " <> errorBundlePretty err)
                  Right _ -> pure ()
                case parseMachineSurface machine of
                  Left err -> error ("minted contest machine failed to parse (bug): " <> errorBundlePretty err)
                  Right _ -> pure ()
                TIO.writeFile factPath factCommit
                TIO.writeFile machinePath machine
                putStrLn ("DIVERGENCE minted as content: " <> factPath <> ", " <> machinePath)
                putStrLn ("  " <> T.unpack subj <> " . " <> T.unpack pred_ <> ": " <> mineLabel <> "=" <> show mv <> " vs " <> peerLabel <> "=" <> show pv)
            )
            (zip [1 :: Int ..] overlapKeys)
    _ -> putStrLn "usage: check-divergence <mine-list-file> <peer-list-file> <output-dir> <mine-label> <peer-label>" >> exitFailure
