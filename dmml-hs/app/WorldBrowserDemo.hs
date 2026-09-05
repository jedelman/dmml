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
-- Extended 2026-09-05 to actually describe co-located subjects, not
-- just name them -- reuses 'DMML.TemplateBank' exactly as
-- @template-compose-fresh-world-demo@ does, no new rendering
-- mechanism. When a subject has NO eligible template (a real content
-- gap, not a code bug -- e.g. this catalog covers @state\/active@
-- smiths but not @state\/training@ ones), the browser doesn't silently
-- say nothing: it emits the "governed catalog expansion" pipeline's
-- own first stage (SPEC.md §19.4) for real -- a structured
-- @requests@/@about@ commit declaring the gap, in the same
-- vocabulary-closed, effect-as-fact shape §19.4 already designed, not
-- free text. Only stage 1 (gap detection -> structured request) is
-- exercised here; offline generation, the review gate, and the
-- approved-citation step are still real, separate, unbuilt work -- the
-- emitted commit is real, parseable DMML (verified via
-- @validate-commit@, see the README), but nothing consumes it yet.
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

import DMML.Ast (GuardClause, MachineStmt, NodeRef (..), Value (..), machineNode, machineTransitions, nodeRefSegments, transitionGuards)
import DMML.Guard (EvalContext (..), availableTransitions)
import DMML.Materialize (WorldSnapshot (..), applyCommit, currentValue, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import DMML.TemplateBank (Template (..), displayNameOf, eligibleTemplates, renderTemplateWith)

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

-- | Small, deliberately narrower catalog than
-- @template-compose-fresh-world-demo@'s -- covers an active oresmith
-- and an active herbalist, but NOT an apprentice-in-training, so that
-- 'browse' has a real gap to demonstrate rather than a fabricated one.
catalog :: [Template]
catalog =
  [ Template
      "smith-at-work"
      (guardsFromText "guard self `a` type/smith\nguard self `state` state/active\nguard self `role` role/oresmith")
      "{subject} works the forge at {attr:worksAt.name}, {attr:role.epithet}."
  , Template
      "herbalist-active"
      (guardsFromText "guard self `a` type/herbalist\nguard self `state` state/active")
      "{subject} tends {attr:worksAt.name} as {attr:role.name}."
  ]

guardsFromText :: Text -> [GuardClause]
guardsFromText src = case parseMachineSurface wrapped of
  Left err -> error ("template guard text failed to parse:\n" <> errorBundlePretty err)
  Right m -> case machineTransitions m of
    (t : _) -> transitionGuards t
    [] -> error "template machine had no transition"
  where
    wrapped =
      "machine tmpl/scratch\n\n  states\n    unused\n\n  transition check()\n"
        <> T.unlines (map ("    " <>) (T.lines src))

-- | Either a rendered description (an eligible template fired) or a
-- real structured content-gap request, printed as a standalone,
-- parseable DMML commit -- the first stage of SPEC.md §19.4's governed
-- catalog-expansion pipeline, exercised for real, not just designed.
describeSubject :: WorldSnapshot -> Int -> Text -> (Text, Maybe Text)
describeSubject snap gapNum subject =
  case eligibleTemplates snap subject catalog of
    (tpl : _) -> (renderTemplateWith snap subject tpl, Nothing)
    [] ->
      ( displayNameOf snap subject <> " (no description on file)"
      , Just (gapRequestCommit gapNum subject)
      )

-- | Hyphens are invalid inside DMML node-ref/identifier segments (a
-- real, previously-hit parse gotcha this session -- @content-gap-1@
-- fails the same way @metal-object@ did earlier) -- every generated
-- segment here is a single run of alphanumerics.
gapRequestCommit :: Int -> Text -> Text
gapRequestCommit n subject =
  T.unlines
    [ "commit contentgap" <> label
    , "  declare relation requests"
    , "  declare relation about"
    , ""
    , "  request/npcgap" <> label <> " `requests` type/descriptiontemplate"
    , "  request/npcgap" <> label <> " `about` " <> subject
    ]
  where
    label = T.pack (show n)

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
          let described = [describeSubject snap n s | (n, s) <- zip [1 ..] others]
          mapM_ (\(line, _) -> putStrLn ("  - " <> T.unpack line)) described
          let gaps = [g | (_, Just g) <- described]
          if null gaps
            then pure ()
            else do
              putStrLn ""
              putStrLn "-- content gap(s) logged as real, unconsumed DMML requests:"
              mapM_ (\g -> TIO.putStr g >> putStrLn "") gaps
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
