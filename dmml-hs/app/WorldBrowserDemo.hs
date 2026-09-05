{-# LANGUAGE OverloadedStrings #-}

-- | The "browser": a player-machine, embedded in the world via a
-- @location@ predicate, surfacing what it perceives and what it can
-- do -- entirely from real facts and real guards, zero generation,
-- zero LLM calls. Answers the design question directly: this whole
-- thing IS buildable in DMML content (the world, the location facts,
-- the action machine's guards) -- what needed new HASKELL was the
-- generic ENUMERATION over "every declared machine's every transition,
-- which ones currently pass" ('DMML.Guard.availableTransitions', new,
-- small, and reused unmodified for any world) -- DMML itself has no
-- construct for "for all machines, for all transitions," only for
-- checking one named transition at a time, same shape as §19.2's own
-- "new evaluation mode" finding for sense-machines.
--
-- Real, deliberate v1 scoping, disclosed not hidden: "what's on stage"
-- here is simple co-location (@EXISTS(other, location, sameRoom)@),
-- not a full sense-machine firing pass restricting perception below
-- "everything in the room" -- that richer sense-gating (SPEC.md §19.2)
-- is real, separate, future work; this proves the browser's basic
-- shape (view + actions, one deterministic pass) without it.
--
-- Usage: world-browser-demo <world.dmml> <machine.dmml> [<machine.dmml> ...]
module Main (main) where

import Data.List (nub)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt, NodeRef (..), Value (..), machineNode, nodeRefSegments)
import DMML.Guard (EvalContext (..), availableTransitions)
import DMML.Materialize (WorldSnapshot (..), applyCommit, currentValue, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import DMML.TemplateBank (displayNameOf)

player :: Text
player = "player/one"

main :: IO ()
main = do
  args <- getArgs
  case args of
    (worldPath : machinePaths) | not (null machinePaths) -> do
      worldSrc <- TIO.readFile worldPath
      case parseCommitSurface worldSrc of
        Left err -> putStrLn (worldPath <> ":\n" <> errorBundlePretty err) >> exitFailure
        Right stmt -> do
          machines <- mapM parseMachineFile machinePaths
          let snap = applyCommit "world" emptySnapshot stmt
              machineMap = Map.fromList [(nodeRefText (machineNode m), m) | m <- machines]
          browse snap machineMap
    _ -> putStrLn "usage: world-browser-demo <world.dmml> <machine.dmml> [<machine.dmml> ...]" >> exitFailure

parseMachineFile :: FilePath -> IO MachineStmt
parseMachineFile path = do
  src <- TIO.readFile path
  case parseMachineSurface src of
    Right m -> pure m
    Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | The whole player-machine loop: locate the player, describe where
-- they are, describe who else is there, enumerate what they can
-- currently do -- one materialized snapshot, no re-firing, no
-- generation, rendered as a CYOA-style page (Jason: "it may be that we
-- don't need a chat surface at all -- we can format this just like an
-- old school choose your own adventure novel").
browse :: WorldSnapshot -> Map.Map Text MachineStmt -> IO ()
browse snap machines = do
  case currentValue (player, "location") snap of
    [] -> putStrLn "The player has no location -- nowhere to browse from."
    ((_label, locationValue) : _) -> do
      let locationNode = nodeTextOf locationValue
          locationName = displayNameOf snap locationNode
      putStrLn (T.unpack locationName)
      putStrLn (replicate (T.length locationName) '=')
      putStrLn ""

      let others = [s | s <- allSubjects, s /= player, atSameLocation s locationNode]
      if null others
        then putStrLn "You are alone here."
        else do
          putStrLn "Also here:"
          mapM_ (\s -> putStrLn ("  - " <> T.unpack (displayNameOf snap s))) others
      putStrLn ""

      let equippedMachines =
            [ nodeTextOf v
            | v <- map snd (currentValue (player, "equips") snap)
            ]
          inScope = Map.filterWithKey (\k _ -> k `elem` equippedMachines) machines
          ctx = EvalContext {ctxSelfNode = player, ctxParams = Map.empty}
          actions = availableTransitions inScope ctx snap
      if null actions
        then putStrLn "You can: nothing, right now."
        else do
          putStrLn "You can:"
          mapM_ (\(_, transition) -> putStrLn ("  * " <> T.unpack transition)) actions
  where
    allSubjects = nub [subj | (subj, _pred) <- Map.keys (snapshotFacts snap)]
    atSameLocation subj room =
      any ((== room) . nodeTextOf) (map snd (currentValue (subj, "location") snap))
    nodeTextOf (ValueNode n) = nodeRefText n
    nodeTextOf _ = ""
