{-# LANGUAGE OverloadedStrings #-}

-- | The cannon: a generator that FIRES ARCHITECTURE. It constructs
-- candidate dungeon rooms as real 'DMML.Ast' values -- never as text --
-- and renders them via 'DMML.Fire.renderFiredMachine' (the exact
-- inverse of 'DMML.Surface.parseMachineSurface', proven to round-trip).
-- Because every emitted machine is a well-typed 'MachineStmt' by
-- construction, the cannon is STRUCTURALLY INCAPABLE of producing
-- ungrammatical DMML -- the formal-grammar guarantee discussed at
-- length: generativity enters here (the generator), grammatical
-- legality is free (AST construction), semantic soundness is still the
-- guard's job downstream (fire-transition filters), and the choice of
-- which generated architecture becomes real stays Jev's.
--
-- The architectural LANGUAGE is a small set of room variants that
-- differ in their RELATIONSHIPS, not their surface shape -- what a room
-- requires to breach and what it yields when breached. That relational
-- variety is what turns a frontier into a real choice for the chooser
-- downstream (build a forge to enable vaults later, spend a move on a
-- spur's treasure now, or push a hall forward), rather than a forced
-- single-option chain.
--
-- Usage:
--   cannon <hall|forge|vault|spur> <newNode> <parentNode>
--   cannon fork <forkNode> <parentNode> <leftFrontier> <rightFrontier>
--   cannon breed <union|chimera|spliceK> <newNode> <a.dmml> <b.dmml> [anchorNode]
--   cannon pool <nodePrefix> <a.dmml> <b.dmml> [anchorNode]
--
-- The four variants and the fork are the cannon as a TEMPLATE STAMP:
-- every shot comes from a shape hand-authored here, so the pool of
-- possible architecture is exactly as large as this file. `breed` and
-- `pool` are the cannon as a BREEDER: they take two machines that
-- already exist -- including two the cannon fired earlier, or two an
-- earlier breeding produced -- and cross them into offspring whose
-- shape is in neither this file nor either parent. That is the
-- difference between a generator with a fixed vocabulary and one whose
-- vocabulary grows with its own output. See "DMML.Recombine" for what
-- crossover means here and which invariants it has to keep.
--
-- `breed` prints one machine; `pool` prints a whole frontier of them,
-- each preceded by a `# <label> <node>` comment line, which is not
-- DMML -- pool output is a catalogue for a chooser to read and split,
-- not a file to feed the parser.
--
-- A room's variant is its RELATIONSHIP to the rest of the dungeon. A
-- fork is the sharper architectural element: one machine with two
-- mutually-exclusive transitions (a single unchosen -> chosen
-- lifecycle lock, so exactly one ever fires), each clearing a
-- DIFFERENT onward frontier. Choosing one strands the other subtree
-- forever -- the one place a chooser downstream faces a real, permanent
-- decision about the dungeon's shape, not just a forced next step.
--
-- Prints one `machine` block to stdout. The machine's own initial
-- `state` fact is NOT emitted here (a machine's current state is
-- mutable world data, not structural definition -- the caller seeds it,
-- same as every other fact-native machine in this project).
module Main (main) where

import Data.Char (isDigit)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast
import DMML.Fire (renderFiredMachine)
import DMML.Recombine (Crossover (..), breed, breedPool, reanchor)
import DMML.Surface (parseMachineSurface)

sp :: Span
sp = Span "cannon"

nr :: Text -> NodeRef
nr = NodeRef . T.splitOn "/"

-- A guard `anchor `pred` obj` over multi-segment literal nodes.
guardFact :: Text -> Text -> Text -> GuardClause
guardFact anchor predicate obj =
  GuardClause
    { guardNegated = False
    , guardExists =
        ExistsExpr
          { existsPattern =
              Pattern
                { patternAnchor = TermNode anchor
                , patternHops = [PatternHop predicate (TermNode obj)]
                }
          , existsSpan = sp
          }
    , guardSpan = sp
    }

-- assert self `pred` obj  (obj a multi-segment literal node)
assertSelf :: Text -> Text -> Effect
assertSelf predicate obj =
  EffectAssert TermSelf (PredIdent predicate) (EffectValueTerm (TermNode obj))

-- assert <node> `pred` obj  (targets some other subject)
assertNode :: Text -> Text -> Text -> Effect
assertNode subj predicate obj =
  EffectAssert (TermNode subj) (PredIdent predicate) (EffectValueTerm (TermNode obj))

lastSeg :: Text -> Text
lastSeg = last . T.splitOn "/"

data Variant = Hall | Forge | Vault | Spur

parseVariant :: String -> Maybe Variant
parseVariant "hall" = Just Hall
parseVariant "forge" = Just Forge
parseVariant "vault" = Just Vault
parseVariant "spur" = Just Spur
parseVariant _ = Nothing

-- | Build a room machine. Every variant shares the same spine -- a
-- `breach` transition, sealed -> open, gated on its parent being
-- cleared, that clears itself so its own children become reachable.
-- The variants diverge only in the RELATIONSHIP layer: extra guards
-- (what else must hold) and extra effects (what breaching yields).
mkRoom :: Variant -> Text -> Text -> MachineStmt
mkRoom variant newNode parent =
  MachineStmt
    { machineNode = nr newNode
    , machineStates = [StateDecl "sealed" sp, StateDecl "open" sp]
    , machineTransitions = [breach]
    , machineSpan = sp
    }
  where
    (extraGuards, extraEffects) = case variant of
      Hall -> ([], [])
      -- forge: no extra requirement; yields a key that vaults need.
      Forge -> ([], [assertNode "vault/keyring" "has" "key/gold"])
      -- vault: requires a key to already exist; yields a sigil.
      Vault -> ([guardFact "vault/keyring" "has" "key/gold"], [assertSelf "yields" ("sigil/" <> lastSeg newNode)])
      -- spur: no extra requirement; yields treasure but clears nothing
      -- onward (see below -- a spur does NOT clear itself, so it is a
      -- genuine dead end: reward at the cost of a foreclosed path).
      Spur -> ([], [assertSelf "yields" ("relic/" <> lastSeg newNode)])
    -- Every variant except a spur clears itself, opening its children.
    clearSelf = case variant of
      Spur -> []
      _ -> [assertSelf "cleared" "mark/yes"]
    breach =
      TransitionDecl
        { transitionIdent = "breach"
        , transitionParams = []
        , transitionFrom = Just "sealed"
        , transitionTo = Just "open"
        , transitionGuards = guardFact parent "cleared" "mark/yes" : extraGuards
        , transitionEffects =
            clearSelf
              ++ extraEffects
              ++ [ EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "open"))
                 , EffectRetract TermSelf [] (PredIdent "state") Nothing
                 ]
        , transitionSpan = sp
        }

-- Connective operators ----------------------------------------------
--
-- Everything above is ARBORESCENT and it is visible in the signatures:
-- @mkRoom variant newNode parent@ -- one parent, one child. The word
-- "parent" is in the API. Every variant hangs a new node off exactly one
-- existing node, so a world built from them is a tree, its frontier is a
-- set of leaves, and "can this keep growing" reduces to "do leaves keep
-- appearing." That is why a single sterile crossover ends a lineage: in
-- a tree, a node that cannot bear children ends a branch.
--
-- A corridor has TWO ENDS. It is not a child of a room; it is an edge
-- between two rooms that already exist, and nothing above can express
-- one. These operators can. Jason, 2026-09-18: "Quarries that fill with
-- rain. Rivers that erode cliffs, exposing clay for bricks. Bricks that
-- build walls. Walls that enclose gardens. Corridors that connect rooms.
-- Towers that rise to a view of... The quarry. See the web densify? A
-- rhizome, not a tree."
--
-- The counting is different, and that is the point. A tree's frontier
-- grows linearly with its nodes; the possible RELATIONS among N nodes
-- grow as N squared. A tree exhausts. A rhizome densifies -- the later
-- something is built, the more there is for it to relate to.
--
-- What keeps that from being noise is that a relation should carry
-- something. A rhizome is not arbitrary connection, it is connection
-- that is not hierarchically constrained, which still leaves the
-- question of what makes any particular connection real. These answer it
-- with flow: a transition's guards are what must come in and its effects
-- are what goes out, so two machines are genuinely connected when one's
-- output meets the other's input. Substances are the lines.

-- | A BRIDGE: a corridor, defined by having two ends.
--
-- Guards on either end being cleared and clears the other, in both
-- directions. That is the operator that makes a world stop being a tree,
-- precisely: it creates a CYCLE in reachability, a second way to arrive
-- somewhere already reachable. Nothing above can produce one, because
-- everything above descends from a single parent.
mkBridge :: Text -> Text -> Text -> MachineStmt
mkBridge bridgeNode endA endB =
  MachineStmt
    { machineNode = nr bridgeNode
    , machineStates = [StateDecl "sealed" sp, StateDecl "open" sp]
    , machineTransitions = [cross "crossFromA" endA endB, cross "crossFromB" endB endA]
    , machineSpan = sp
    }
  where
    cross name from to =
      TransitionDecl
        { transitionIdent = name
        , transitionParams = []
        , transitionFrom = Just "sealed"
        , transitionTo = Just "open"
        , transitionGuards = [guardFact from "cleared" "mark/yes"]
        , transitionEffects =
            [ assertSelf "cleared" "mark/yes"
            , assertNode to "cleared" "mark/yes"
            , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "open"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

-- | A FEED: a transformer. Consumes one unit of one substance and
-- produces one unit of another, IN PLACE -- the same unit node changes
-- what it is. Clay becomes brick; the matter is conserved and only its
-- kind changes.
--
-- This is the operator @?binder@ was built for. The guard finds a unit,
-- and the effects spend THAT unit rather than some unit -- which is what
-- makes a transformation a real transformation rather than an assertion
-- about inventory. It resets so it can run again: a mill that could fire
-- one brick would be a room with extra steps.
mkFeed :: Text -> Text -> Text -> MachineStmt
mkFeed millNode fromKind toKind =
  MachineStmt
    { machineNode = nr millNode
    , machineStates = [StateDecl "idle" sp, StateDecl "working" sp]
    , machineTransitions = [transform, reset]
    , machineSpan = sp
    }
  where
    transform =
      TransitionDecl
        { transitionIdent = "transform"
        , transitionParams = []
        , transitionFrom = Just "idle"
        , transitionTo = Just "working"
        , transitionGuards =
            [ GuardClause False (ExistsExpr (Pattern (TermBind "unit") [PatternHop "is" (TermNode fromKind)]) sp) sp
            ]
        , transitionEffects =
            [ EffectRetract (TermBind "unit") [] (PredIdent "is") (Just (EffectValueTerm (TermNode fromKind)))
            -- Same predicate as the retract, deliberately: the unit's
            -- KIND changes, it does not gain a second attribute. Checked
            -- rather than assumed -- a retract lowers into the commit's
            -- `consumes` block and an assert into its facts, so there is
            -- no duplicate (subject, predicate) collision. An earlier
            -- draft dodged this with a separate `becomes` predicate and
            -- thereby broke the only thing that matters here: a second
            -- mill must be able to consume the first's output.
            , EffectAssert (TermBind "unit") (PredIdent "is") (EffectValueTerm (TermNode toKind))
            , assertSelf "cleared" "mark/yes"
            , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "working"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }
    reset =
      TransitionDecl
        { transitionIdent = "reset"
        , transitionParams = []
        , transitionFrom = Just "working"
        , transitionTo = Just "idle"
        , transitionGuards = []
        , transitionEffects =
            [ EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "idle"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

-- | A REPLENISH: rain. Produces a fresh unit of a substance, consuming
-- nothing.
--
-- The cycle-closer, and the only operator here that can make a world
-- sustain rather than run down. Everything else moves matter along; this
-- puts some back. A quarry that fills with rain is not a metaphor for
-- sustainability -- it is literally a cycle in the substance-flow graph,
-- which is the structural difference between a world that depletes and
-- one that does not.
--
-- The new unit's node comes from a @$param@: the caller names what
-- arrives, the same division of labour every other minting in this
-- project uses. An effect whose subject is a @$param@ brings a node into
-- existence the instant it resolves (open-world -- see
-- 'DMML.Ast.Effect').
mkReplenish :: Text -> Text -> Text -> MachineStmt
mkReplenish node kind source =
  MachineStmt
    { machineNode = nr node
    , machineStates = [StateDecl "gathering" sp, StateDecl "spent" sp]
    , machineTransitions = [fall, gather]
    , machineSpan = sp
    }
  where
    fall =
      TransitionDecl
        { transitionIdent = "fall"
        , transitionParams = ["unit"]
        , transitionFrom = Just "gathering"
        , transitionTo = Just "spent"
        , transitionGuards = [guardFact source "cleared" "mark/yes"]
        , transitionEffects =
            [ EffectAssert (TermParam "unit") (PredIdent "is") (EffectValueTerm (TermNode kind))
            , assertSelf "cleared" "mark/yes"
            , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "spent"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }
    gather =
      TransitionDecl
        { transitionIdent = "gather"
        , transitionParams = []
        , transitionFrom = Just "spent"
        , transitionTo = Just "gathering"
        , transitionGuards = []
        , transitionEffects =
            [ EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "gathering"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

-- | A VISTA: the tower that rises to a view of the quarry.
--
-- Relates two existing places and moves nothing between them. Requires
-- BOTH to be real before the view exists, and changes neither. This is
-- the reference half of the distinction this project drew earlier the
-- same day -- copies act, references remember -- and it needs no
-- primitive at all, because a fact with a node value already IS an edge.
--
-- It densifies without flowing, which is worth having on its own: a
-- rhizome whose every line carried substance would be a supply chain,
-- not a world.
mkVista :: Text -> Text -> Text -> MachineStmt
mkVista node anchor target =
  MachineStmt
    { machineNode = nr node
    , machineStates = [StateDecl "sealed" sp, StateDecl "open" sp]
    , machineTransitions = [climb]
    , machineSpan = sp
    }
  where
    climb =
      TransitionDecl
        { transitionIdent = "climb"
        , transitionParams = []
        , transitionFrom = Just "sealed"
        , transitionTo = Just "open"
        , transitionGuards =
            [guardFact anchor "cleared" "mark/yes", guardFact target "cleared" "mark/yes"]
        , transitionEffects =
            [ EffectAssert TermSelf (PredIdent "overlooks") (EffectValueTerm (TermNode target))
            , assertSelf "cleared" "mark/yes"
            , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "open"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

-- | A fork: one machine, two mutually-exclusive transitions sharing a
-- single unchosen -> chosen lock. Each clears a different onward
-- frontier node; the from->to lifecycle (not a negated guard, which the
-- retroconsistency gate would refuse) is what makes them exclusive.
mkFork :: Text -> Text -> Text -> Text -> MachineStmt
mkFork forkNode parent leftFrontier rightFrontier =
  MachineStmt
    { machineNode = nr forkNode
    , machineStates = [StateDecl "unchosen" sp, StateDecl "chosen" sp]
    , machineTransitions = [go "goLeft" leftFrontier, go "goRight" rightFrontier]
    , machineSpan = sp
    }
  where
    go name frontier =
      TransitionDecl
        { transitionIdent = name
        , transitionParams = []
        , transitionFrom = Just "unchosen"
        , transitionTo = Just "chosen"
        , transitionGuards = [guardFact parent "cleared" "mark/yes"]
        , transitionEffects =
            [ assertNode frontier "cleared" "mark/yes"
            , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "chosen"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

-- | Read a parent machine off disk. A parent is ordinary Surface DMML
-- -- whatever this cannon printed earlier, or any hand-authored machine
-- -- so the breeder's input language is exactly its output language and
-- nothing special has to be persisted to make a machine breedable.
loadParent :: FilePath -> IO MachineStmt
loadParent path = do
  txt <- TIO.readFile path
  case parseMachineSurface txt of
    Left err -> do
      hPutStrLn stderr ("cannon: cannot parse " <> path <> ":")
      hPutStrLn stderr (errorBundlePretty err)
      exitFailure
    Right m -> pure m

parseCrossover :: String -> Maybe Crossover
parseCrossover "union" = Just Union
parseCrossover "chimera" = Just Chimera
parseCrossover s
  | ("splice", k) <- splitAt 6 s
  , not (null k)
  , all isDigit k =
      Just (Splice (read k))
parseCrossover _ = Nothing

-- | Apply the optional re-anchor, if the caller named one.
place :: [String] -> MachineStmt -> MachineStmt
place [anchor] = reanchor (T.pack anchor)
place _ = id

main :: IO ()
main = do
  args <- getArgs
  case args of
    ("breed" : modeStr : newNode : aPath : bPath : rest)
      | Just mode <- parseCrossover modeStr
      , length rest <= 1 -> do
          a <- loadParent aPath
          b <- loadParent bPath
          case breed sp mode (nr (T.pack newNode)) a b of
            Left err -> hPutStrLn stderr ("cannon: cannot breed: " <> show err) >> exitFailure
            Right child -> TIO.putStr (renderFiredMachine (place rest child))
    ("pool" : prefix : aPath : bPath : rest)
      | length rest <= 1 -> do
          a <- loadParent aPath
          b <- loadParent bPath
          case breedPool sp (nr (T.pack prefix)) a b of
            [] -> hPutStrLn stderr "cannon: nothing to breed from this pair" >> exitFailure
            candidates ->
              mapM_
                ( \(label, child) -> do
                    let placed = place rest child
                    TIO.putStrLn ("# " <> label <> " " <> T.intercalate "/" (nodeRefSegments (machineNode placed)))
                    TIO.putStr (renderFiredMachine placed)
                )
                candidates
    ["bridge", node, endA, endB] ->
      TIO.putStr (renderFiredMachine (mkBridge (T.pack node) (T.pack endA) (T.pack endB)))
    ["feed", node, fromKind, toKind] ->
      TIO.putStr (renderFiredMachine (mkFeed (T.pack node) (T.pack fromKind) (T.pack toKind)))
    ["replenish", node, kind, source] ->
      TIO.putStr (renderFiredMachine (mkReplenish (T.pack node) (T.pack kind) (T.pack source)))
    ["vista", node, anchor, target] ->
      TIO.putStr (renderFiredMachine (mkVista (T.pack node) (T.pack anchor) (T.pack target)))
    ["fork", forkNode, parent, left, right] ->
      TIO.putStr (renderFiredMachine (mkFork (T.pack forkNode) (T.pack parent) (T.pack left) (T.pack right)))
    [v, newNode, parent] ->
      case parseVariant v of
        Just variant -> TIO.putStr (renderFiredMachine (mkRoom variant (T.pack newNode) (T.pack parent)))
        Nothing -> putStrLn ("cannon: unknown variant " <> v <> " (want hall|forge|vault|spur)") >> exitFailure
    _ ->
      putStrLn "usage: cannon <hall|forge|vault|spur> <newNode> <parentNode>"
        >> putStrLn "       cannon fork <forkNode> <parentNode> <leftFrontier> <rightFrontier>"
        >> putStrLn "       cannon breed <union|chimera|spliceK> <newNode> <a.dmml> <b.dmml> [anchorNode]"
        >> putStrLn "       cannon pool <nodePrefix> <a.dmml> <b.dmml> [anchorNode]"
        >> putStrLn "  connective (two ends, not one parent -- a rhizome, not a tree):"
        >> putStrLn "       cannon bridge <node> <endA> <endB>        -- a corridor; makes a reachability CYCLE"
        >> putStrLn "       cannon feed <node> <fromKind> <toKind>    -- a mill; clay becomes brick, in place"
        >> putStrLn "       cannon replenish <node> <kind> <source>   -- rain; produces, consuming nothing"
        >> putStrLn "       cannon vista <node> <anchor> <target>     -- a tower; relates without flowing"
        >> exitFailure
