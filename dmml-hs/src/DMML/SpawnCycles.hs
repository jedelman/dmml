{-# LANGUAGE OverloadedStrings #-}

-- | Flags, never blocks, a machine-production loop -- the check
-- 'DMML.Ast.Effect'\'s new 'DMML.Ast.EffectSpawn' constructor makes
-- newly necessary: once a transition can spawn a fresh instance of an
-- existing machine template, a template whose own transitions spawn
-- back to itself (directly, or through a chain of other templates'
-- spawn effects) can keep producing instances of its own lineage
-- forever. Jason's own framing: "a reproductive system for universes,"
-- flagged the same breath as asking for spawn itself, alongside "flag
-- if not error."
--
-- Deliberately a STATIC, ADVISORY check over the machine DEFINITIONS
-- alone, at the same layer as @validate-commit@\/@check-declared@\/
-- @retro-gate@ -- separate tools a caller runs, not something
-- 'DMML.Fire.fireTransition' enforces itself. Two real reasons, not
-- just "matching existing style":
--
--   1. A template naming itself in its own spawn effect is not
--      automatically non-terminating -- a guard elsewhere in the same
--      transition could make the second spawn unreachable in practice
--      (e.g. a population cap fact the guard checks). Hard-refusing at
--      fire time would be wrong more often than right; this can only
--      see the shape of the DEFINITIONS, never a specific run's guard
--      outcomes.
--   2. Jason asked for a flag, not an error, explicitly.
--
-- A caller that DOES want a hard stop (the jev-driver-demo loop, a
-- human reviewing a seed world before it runs) can still treat any
-- non-empty 'detectSpawnCycles' result as fatal -- that's a caller
-- policy layered on top of an honest report, not something this module
-- decides on anyone's behalf.
module DMML.SpawnCycles
  ( spawnEdges
  , detectSpawnCycles
  ) where

import Data.List (nub)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | Every (spawning machine, named template) edge declared anywhere in
-- the known machine set's transitions -- a static property of the
-- machine DEFINITIONS, independent of whether any transition named
-- here has ever actually fired, or ever legally could with the
-- snapshot a real run has in hand.
spawnEdges :: Map.Map Text MachineStmt -> [(Text, Text)]
spawnEdges machines =
  [ (nodeRefText (machineNode m), nodeRefText templateRef)
  | m <- Map.elems machines
  , t <- machineTransitions m
  , EffectSpawn _ templateRef <- transitionEffects t
  ]

-- | Every distinct cycle in the spawn graph, each reported as the
-- ordered node-text list that walks it, first node repeated as the
-- last (so a direct self-spawn -- a machine's own transition names its
-- own node as the template -- renders as the 2-element @[x, x]@, not
-- specially hidden: it is the simplest real case of exactly the risk
-- this exists to flag). Duplicate reports of the same underlying cycle
-- reached by different starting points are collapsed by normalizing
-- each cycle to start at its lexicographically smallest node before
-- deduplicating -- a caller should see each real cycle once, not once
-- per node that happens to reach it.
--
-- The graph a real seed world builds is expected to be small (a
-- handful of producer-machine templates, not thousands), so this
-- favors a simple, directly-readable exploration over an asymptotically
-- optimal cycle-finding algorithm (e.g. Tarjan's SCC) it doesn't need
-- yet.
detectSpawnCycles :: Map.Map Text MachineStmt -> [[Text]]
detectSpawnCycles machines = nub (map normalize (concatMap (walkFrom []) allNodes))
  where
    edges = spawnEdges machines
    allNodes = nub (concatMap (\(a, b) -> [a, b]) edges)
    children n = [b | (a, b) <- edges, a == n]

    -- | @path@ is every node walked so far, in order, NOT including
    -- @node@ itself -- each step extends it by exactly one before
    -- checking the next node against it, so a cycle is caught the
    -- instant a child re-enters the walked set, never one node late or
    -- early.
    walkFrom :: [Text] -> Text -> [[Text]]
    walkFrom path node = concatMap step (children node)
      where
        path' = path ++ [node]
        step child
          | child `elem` path' = [dropWhile (/= child) path' ++ [child]]
          | otherwise = walkFrom path' child

    -- | Rotate a cycle (dropping its duplicated closing node first) so
    -- it starts at its smallest element, then re-close it -- makes two
    -- textually different walks of the same cycle compare equal.
    normalize :: [Text] -> [Text]
    normalize cyc =
      let open = init cyc -- drop the repeated closing node
          smallest = minimum open
          (before, after) = break (== smallest) open
          rotated = after ++ before
       in rotated ++ [smallest]
