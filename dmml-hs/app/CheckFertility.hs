{-# LANGUAGE OverloadedStrings #-}

-- | Predict, WITHOUT running anything, how long a seed world can keep
-- producing reachable architecture under @app\/Cannon.hs@'s breeding.
--
-- Written 2026-09-18 after a real run built SIXTY rooms and fired twice.
-- Every one of those rooms guarded on the previous room being @cleared@,
-- and none of them ever asserted @self \\`cleared\\`@ -- so the whole
-- 60-room chain was unreachable the moment the second one was minted.
-- The loop could not tell, because from inside a round "a machine was
-- minted" and "a machine that can ever fire was minted" look identical.
--
-- == Why this is decidable at all
--
-- Crossover never invents a transition. 'DMML.Recombine' only selects
-- and recombines what the parents already have: 'Union' takes their
-- transitions, 'Splice' takes a prefix and a suffix, and 'Chimera' pairs
-- one parent's guard list with the other's effect list. So every
-- transition reachable from a seed is drawn from
-- @GuardLists(seed) x EffectLists(seed)@ -- a finite set. Quotient by
-- node renaming (which is the only thing 'breed' introduces that is not
-- already in a parent) and the space of reachable machine STRUCTURES is
-- finite.
--
-- The driver's breeding policy is deterministic besides: parent A is the
-- newest offspring, parent B cycles the seed machines, and the crossover
-- mode cycles. A deterministic walk over a finite space is eventually
-- periodic, so simulating until a structure repeats answers the question
-- exactly rather than approximately. No branching-process estimate, no
-- sampling -- the cycle either contains a fertile machine or it does
-- not.
--
-- == What "fertile" means here, precisely
--
-- Fertility is a property of the ATTACHMENT CONVENTION, not of the
-- grammar. @app\/Cannon.hs@ anchors every room it fires with
-- @guard \\<parent\\> \\`cleared\\` mark\/yes@, so a room can be built upon
-- only once something asserts @cleared@ on it -- and the only thing that
-- ever will is the room itself, via @assert self \\`cleared\\`@. A machine
-- with no such effect is a DEAD END: reachable, possibly interesting,
-- but nothing can ever be anchored beyond it.
--
-- That is exactly what a chimera of (room x fork) loses. A fork clears a
-- PATH node rather than itself, so an offspring that takes the fork's
-- effects inherits "opens a way" without inheriting "is itself cleared."
-- One such crossover ends a lineage, and this tool says which one and
-- when.
--
-- Usage: check-fertility <seed.dmml>... [--generations N]
module Main (main) where

import Data.Maybe (mapMaybe)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast
import DMML.Recombine (Crossover (..), breed, crossoverLabel)
import DMML.Surface (parseMachineSurface)

sp :: Span
sp = Span "check-fertility"

nodeText :: MachineStmt -> Text
nodeText = T.intercalate "/" . nodeRefSegments . machineNode

-- | Does firing anything on this machine make it anchorable?
--
-- The one structural question that decides whether a lineage continues,
-- under the cannon's own attachment convention.
selfClearing :: MachineStmt -> Bool
selfClearing m =
  or
    [ True
    | t <- machineTransitions m
    , EffectAssert TermSelf (PredIdent "cleared") _ <- transitionEffects t
    ]

-- | A machine's structure with its identity erased, so two offspring
-- that differ only by generation number compare equal. This is what
-- makes the walk terminate: without it every generation looks new
-- forever, because 'breed' renames.
shape :: MachineStmt -> Text
shape m =
  T.intercalate
    "|"
    ( [T.pack (show (map stateIdent (machineStates m)))]
        ++ [ T.pack (show (transitionFrom t, transitionTo t, length (transitionGuards t), map scrub (transitionEffects t)))
           | t <- machineTransitions m
           ]
    )
  where
    -- Effects mentioning the machine's own derived nodes are normalized,
    -- since those differ by name across generations and by nothing else.
    scrub e = T.replace (lastSeg (nodeText m)) "SELF" (T.pack (show e))
    lastSeg = last . T.splitOn "/"

-- | The driver's own breeding policy, replayed without a world: parent A
-- is the newest offspring, parent B cycles the seeds, the mode cycles.
-- Kept deliberately in step with @examples\/jev-driver-demo\/driver.py@'s
-- @plan_extension@ -- if that policy changes, this prediction is only as
-- good as its agreement with it, which is a real coupling and worth
-- saying out loud rather than discovering later.
walk :: [MachineStmt] -> Int -> [(Int, Text, Text, Bool, MachineStmt)]
walk seeds n = go 0 (last seeds) []
  where
    modes = [Union, Chimera, Splice 1]
    go i a seen
      | i >= n = []
      | otherwise =
          let b = seeds !! (i `mod` length seeds)
              mode = modes !! (i `mod` length modes)
              child = NodeRef ["room", "g" <> T.pack (show i)]
           in case breed sp mode child a b of
                Left _ -> []
                Right off ->
                  let sh = shape off
                      row = (i, crossoverLabel mode <> " x " <> nodeText b, sh, selfClearing off, off)
                   in if sh `elem` seen
                        then [row]
                        else row : go (i + 1) off (sh : seen)

main :: IO ()
main = do
  args <- getArgs
  let (paths, gens) = parseArgs args
  if null paths
    then putStrLn "usage: check-fertility <seed.dmml>... [--generations N]" >> exitFailure
    else do
      seeds <- mapM load paths
      putStrLn "seed machines:"
      mapM_
        (\m -> putStrLn ("  " <> T.unpack (nodeText m) <> fert (selfClearing m)))
        seeds
      putStrLn ""
      let rows = walk seeds gens
          firstSterile = [i | (i, _, _, False, _) <- rows]
      putStrLn ("bred lineage (parent A = newest offspring, B cycles seeds, mode cycles), " <> show (length rows) <> " generation(s):")
      mapM_
        (\(i, how, _, f, _) -> putStrLn ("  g" <> show i <> "  " <> T.unpack how <> fert f))
        rows
      putStrLn ""
      case firstSterile of
        [] ->
          putStrLn $
            "check-fertility: FERTILE -- every generation in the cycle clears itself, so a run is\n"
              <> "bounded by budget rather than by architecture."
        (k : _) -> do
          putStrLn $
            "check-fertility: STERILE at generation g"
              <> show k
              <> " -- that offspring never asserts `self `cleared``, so\n"
              <> "nothing can ever be anchored beyond it. Under the cannon's attachment convention\n"
              <> "(guard <parent> `cleared` mark/yes) the lineage ends there: later machines will\n"
              <> "still be MINTED, but they guard on a node that can never be cleared and so can\n"
              <> "never fire. Expect growth without reachability."
          putStrLn ""
          putStrLn "The crossover that does it, in full:"
          mapM_
            (\(_, _, _, _, m) -> TIO.putStr (T.unlines (map ("  " <>) (T.lines (render m)))))
            (take 1 [r | r@(i, _, _, False, _) <- rows, i == k])
          exitFailure
  where
    fert True = "   [fertile -- clears itself]"
    fert False = "   [STERILE -- never clears itself]"

    render m =
      T.unlines $
        ["machine " <> nodeText m]
          ++ ["  states " <> T.intercalate ", " (map stateIdent (machineStates m))]
          ++ concat
            [ ["  transition " <> transitionIdent t]
                ++ ["    guard ..." | not (null (transitionGuards t))]
                ++ ["    " <> T.pack (show e) | e <- transitionEffects t]
            | t <- machineTransitions m
            ]

    parseArgs as = (mapMaybe keep (zip as (drop 1 as ++ [""])), gensOf as)
      where
        keep (a, _) = if take 2 a == "--" then Nothing else Just a
        gensOf ("--generations" : n : _) = read n
        gensOf (_ : more) = gensOf more
        gensOf [] = 40

    load path = do
      src <- TIO.readFile path
      case parseMachineSurface src of
        Right m -> pure m
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure

