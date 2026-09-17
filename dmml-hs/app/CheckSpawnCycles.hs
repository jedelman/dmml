{-# LANGUAGE OverloadedStrings #-}

-- | CLI: flags (never blocks) a machine-production cycle across a set
-- of real machine files -- 'DMML.SpawnCycles'\'s own doc comment
-- explains why this is advisory, at the same layer as
-- @validate-commit@\/@check-declared@, rather than something
-- @fire-transition@ itself refuses. Exit 0 (no cycle found) or 1
-- (lists every distinct cycle found) -- a caller that wants a hard
-- stop (a CI check, a driver loop's pre-flight) can treat exit 1 as
-- fatal; this binary itself never decides that on anyone's behalf.
--
-- Usage: check-spawn-cycles <machine.dmml> [<machine.dmml> ...]
--
-- UNCOMPILED: written in a sandbox with no megaparsec available (see
-- this change's own PR description) -- DMML.SpawnCycles itself (the
-- only real logic here) was compiled AND run for real; this thin CLI
-- wrapper around it, which needs DMML.Surface to parse machine files,
-- was not.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt, NodeRef, machineNode, nodeRefSegments)
import DMML.SpawnCycles (detectSpawnCycles)
import DMML.Surface (parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: check-spawn-cycles <machine.dmml> [<machine.dmml> ...]" >> exitFailure
    paths -> do
      machines <- mapM parseMachineFile paths
      let machineMap = Map.fromList [(nodeRefText (machineNode m), m) | m <- machines]
          cycles = detectSpawnCycles machineMap
      if null cycles
        then putStrLn "check-spawn-cycles: OK -- no machine-production cycle found"
        else do
          putStrLn "check-spawn-cycles: FLAGGED -- machine-production cycle(s) found:"
          mapM_ (\c -> putStrLn ("  " <> T.unpack (T.intercalate " -> " c))) cycles
          putStrLn "not necessarily a bug -- a guard elsewhere may make the loop's next iteration unreachable in practice."
          exitFailure
  where
    parseMachineFile :: FilePath -> IO MachineStmt
    parseMachineFile path = do
      src <- TIO.readFile path
      case parseMachineSurface src of
        Right m -> pure m
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"
