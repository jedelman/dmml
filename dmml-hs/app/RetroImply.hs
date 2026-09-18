{-# LANGUAGE OverloadedStrings #-}

-- | CLI: run retroaction. Given a machine, one of its transitions, and a
-- world that does NOT currently satisfy that transition's guards, print
-- the commit of facts that WOULD need to already exist for it to have
-- legitimately fired -- 'DMML.Retroconsistency.retroconsistency' +
-- 'renderImpliedCommit', Jason's "if a forest is depleted, there must be
-- someone who depleted it... fill them in in a commit as long as it's
-- consistent," exposed as a real tool instead of a hardcoded demo.
--
-- This is how you "build the machines but work backwards": author a
-- transition whose guards DESCRIBE the history a present state implies
-- (a party reading the traces of a prior attempt), point it at a world
-- holding only the present, and retroaction synthesizes the implied
-- past as a real, re-parseable DMML commit -- minting the prior
-- adventurer, their end, their path, as the facts the present requires.
--
-- Usage: retro-imply <machine.dmml> <transition> [--world <f.dmml>]... [--param k=v]...
--   Exit 0 and print the implied commit when history is synthesized;
--   exit 0 and say so when the guards already hold (nothing to imply);
--   exit 1 when a guard is irreconcilable (an unbound anchor, or a
--   negated guard blocked by a real fact -- retroaction can only add,
--   never retract).
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt, MachineStmt, machineNode, nodeRefSegments)
import DMML.Guard (EvalContext (..))
import DMML.Materialize (applyCommits)
import DMML.Retroconsistency (RetroResult (..), renderImpliedCommit, retroconsistency)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: MachineStmt -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments . machineNode

data Args = Args
  { argMachine :: FilePath
  , argTransition :: Text
  , argWorlds :: [FilePath]
  , argParams :: [(Text, Text)]
  }

parseArgs :: [String] -> Either String Args
parseArgs (m : tr : rest) = go rest [] []
  where
    go [] ws ps = Right (Args m (T.pack tr) (reverse ws) (reverse ps))
    go ("--world" : f : more) ws ps = go more (f : ws) ps
    go ("--param" : kv : more) ws ps = case break (== '=') kv of
      (k, '=' : v) -> go more ws ((T.pack k, T.pack v) : ps)
      _ -> Left ("--param expects name=value, got " <> kv)
    go (other : _) _ _ = Left ("unrecognized argument " <> other)
parseArgs _ = Left "usage: retro-imply <machine.dmml> <transition> [--world <f.dmml>]... [--param k=v]..."

classify :: FilePath -> Text -> IO (Either CommitStmt MachineStmt)
classify path src = case parseCommitSurface src of
  Right c -> pure (Left c)
  Left commitErr -> case parseMachineSurface src of
    Right m -> pure (Right m)
    Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure >> error "unreachable"

main :: IO ()
main = do
  raw <- getArgs
  case parseArgs raw of
    Left err -> putStrLn err >> exitFailure
    Right args -> do
      machineSrc <- TIO.readFile (argMachine args)
      machine <- case parseMachineSurface machineSrc of
        Right m -> pure m
        Left e -> putStrLn (argMachine args <> ":\n" <> errorBundlePretty e) >> exitFailure >> error "unreachable"
      worldSrcs <- mapM TIO.readFile (argWorlds args)
      classified <- mapM (uncurry classify) (zip (argWorlds args) worldSrcs)
      let commits = [c | Left c <- classified]
          snap = applyCommits "world" commits
          ctx = EvalContext {ctxSelfNode = nodeRefText machine, ctxParams = Map.fromList (argParams args)}
      case retroconsistency machine (argTransition args) ctx snap of
        Nothing -> putStrLn ("retro-imply: no transition named " <> T.unpack (argTransition args) <> " on this machine") >> exitFailure
        Just AlreadyConsistent -> putStrLn "retro-imply: already consistent -- every guard already holds, nothing to imply"
        Just (Irreconcilable msg) -> putStrLn ("retro-imply: irreconcilable -- " <> T.unpack msg) >> exitFailure
        Just (Implied facts) -> TIO.putStr (renderImpliedCommit "retroactively" facts)
