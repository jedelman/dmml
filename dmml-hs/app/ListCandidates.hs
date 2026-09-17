{-# LANGUAGE OverloadedStrings #-}

-- | CLI: lists every (machine, transition) pair discoverable directly
-- from a set of real commit files -- 'DMML.MachineFacts.candidateTransitions'
-- applied to a snapshot built from @--world@ commits, the exact
-- convention @app/FireTransition.hs@ already uses. This is the
-- "candidates are hand-authored, not auto-enumerated" gap
-- @examples/jev-driver-demo/README.md@ originally flagged, closed for
-- exactly the population that's fact-native (typically something
-- @DMML.Fire.renderFiredCommits@ produced, from an @EffectSpawn@ or
-- deliberate @encodeMachine@ authoring) -- a hand-authored Surface-text
-- machine that was never encoded this way is correctly invisible; see
-- @candidateTransitions@'s own doc comment.
--
-- Usage: list-candidates <world.dmml> [<world.dmml> ...]
--
-- Prints one line per (machine, transition, param-names) triple, or
-- "no candidates found" (exit 0 either way -- an empty result is not
-- an error, it just means nothing fact-native is in scope yet).
--
-- UNCOMPILED: written without megaparsec available (see this change's
-- PR description) -- DMML.MachineFacts.candidateTransitions itself
-- (the only real logic here) was compiled AND run for real, against a
-- snapshot built the identical way this binary builds one; this thin
-- CLI wrapper, which needs DMML.Surface to parse --world files, was
-- not.
module Main (main) where

import qualified Data.ByteString as BS
import Data.List (sortOn)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt, NodeRef, TransitionDecl (..), machineNode, nodeRefSegments)
import DMML.LocalIdentity (localFileRef)
import DMML.MachineFacts (candidateTransitions)
import DMML.Materialize (IdentifiedCommit (..), applyIdentifiedCommits)
import DMML.Surface (parseCommitSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: list-candidates <world.dmml> [<world.dmml> ...]" >> exitFailure
    paths -> do
      identified <- mapM parseWorldFile paths
      let snap = applyIdentifiedCommits "world" identified
          found =
            sortOn
              (\(mNode, t, _) -> (mNode, transitionIdent t))
              [(nodeRefText (machineNode m), t, transitionParams t) | (m, t) <- candidateTransitions snap]
      if null found
        then putStrLn "list-candidates: no fact-native machines found in scope"
        else mapM_ printCandidate found
  where
    printCandidate (mNode, t, params) =
      putStrLn
        ( T.unpack mNode
            <> " "
            <> T.unpack (transitionIdent t)
            <> "("
            <> T.unpack (T.intercalate ", " params)
            <> ")"
        )

    parseWorldFile :: FilePath -> IO IdentifiedCommit
    parseWorldFile path = do
      raw <- BS.readFile path
      case parseCommitSurface (TE.decodeUtf8 raw) of
        Right c -> pure IdentifiedCommit {icRef = localFileRef path raw, icCommit = c}
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"
