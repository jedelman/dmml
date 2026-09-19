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
-- == Two different questions, because there are two different shapes
--
-- Extended 2026-09-18 (Jason: "we're missing a crucial transition --
-- relationship... See the web densify? A rhizome, not a tree"), because
-- the lineage analysis above answers a question about a TREE, and a
-- world is not only a tree.
--
-- The variants in @app\/Cannon.hs@ are arborescent by signature --
-- @mkRoom variant newNode parent@, one parent, one child -- so a world
-- built from them has leaves, and "can it keep growing" reduces to "do
-- leaves keep appearing." That is the walk above.
--
-- The connective operators (@bridge@, @feed@, @replenish@, @vista@) take
-- nodes that ALREADY EXIST and relate them. A corridor has two ends. A
-- world built with those has no leaves to run out of: the possible
-- relations among N nodes grow as N squared, so it densifies rather than
-- exhausts. Asking "do leaves keep appearing" of such a world is asking
-- the wrong question of the wrong shape.
--
-- The right question there is about FLOW, and it has an equally sharp
-- answer. A transition's guards are what must come in; its effects are
-- what goes out; so a set of machines induces a directed graph on
-- SUBSTANCES, and:
--
--   * a DAG runs down -- every substance traces back to a finite stock,
--     and the world depletes no matter how much architecture exists;
--   * a graph with a CYCLE sustains.
--
-- "Quarries that fill with rain" is not a metaphor for sustainability.
-- It is literally a cycle in that graph, and cycle-detection on a finite
-- graph is trivial -- so the rhizomatic question is as decidable as the
-- arborescent one, just about something else.
--
-- Both sections are reported because a seed needs both: architecture
-- that can be extended, and flows that do not all terminate.
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

import Data.List (nub, sort)
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

-- Flow analysis -------------------------------------------------------

-- | A substance, as this corpus actually writes one: a (predicate,
-- object) pair asserted or retracted about a UNIT -- a @?binder@ or
-- @$param@ subject, never @self@ and never a literal place.
--
-- That distinction is the whole trick. @assert self \`cleared\`
-- mark\/yes@ is a machine saying something about itself; @retract ?unit
-- \`is\` clay\/raw@ is matter moving. Only the second is a flow, and
-- only flows can cycle.
substanceOf :: Effect -> Maybe Text
substanceOf (EffectAssert subj (PredIdent p) (EffectValueTerm (TermNode o)))
  | isUnit subj = Just (p <> " " <> o)
substanceOf (EffectRetract subj [] (PredIdent p) (Just (EffectValueTerm (TermNode o))))
  | isUnit subj = Just (p <> " " <> o)
substanceOf _ = Nothing

isUnit :: PatternTerm -> Bool
isUnit (TermBind _) = True
isUnit (TermParam _) = True
isUnit _ = False

isRetract :: Effect -> Bool
isRetract EffectRetract {} = True
isRetract _ = False

-- | (consumed, produced) for one transition.
flowOf :: TransitionDecl -> ([Text], [Text])
flowOf t =
  ( [x | e <- transitionEffects t, isRetract e, Just x <- [substanceOf e]]
  , [x | e <- transitionEffects t, not (isRetract e), Just x <- [substanceOf e]]
  )

-- | Every substance edge the machine set induces, plus the substances
-- produced with nothing consumed (the sources -- rain).
flowGraph :: [MachineStmt] -> ([(Text, Text)], [Text], [Text])
flowGraph ms = (nub edges, nub sources, nub allSubs)
  where
    flows = [flowOf t | m <- ms, t <- machineTransitions m]
    edges = [(c, p) | (cs, ps) <- flows, c <- cs, p <- ps]
    sources = concat [ps | (cs, ps) <- flows, null cs]
    allSubs = concat [cs ++ ps | (cs, ps) <- flows]

-- | Any cycle in the substance graph, as a reachable-from-itself list.
cyclic :: [(Text, Text)] -> [Text]
cyclic edges = [s | s <- flowNodes edges, s `elem` reachFrom edges s]

flowNodes :: [(Text, Text)] -> [Text]
flowNodes edges = nub (map fst edges ++ map snd edges)

reachFrom :: [(Text, Text)] -> Text -> [Text]
reachFrom edges s = go [s] []
  where
    go [] seen = seen
    go (x : xs) seen =
      let nexts = [b | (a, b) <- edges, a == x, b `notElem` seen]
       in go (xs ++ nexts) (seen ++ nexts)

-- Topology ------------------------------------------------------------------
--
-- Jason, 2026-09-19: "fertility is a topological property isn't it."
-- Substantially yes, and more literally than it first sounds -- with one
-- correction that turns out to carry the whole distinction this module
-- draws.
--
-- The correction: it is NOT a property of the underlying undirected
-- space. Take @clay -> brick@, @clay -> tile@, @brick -> wall@,
-- @tile -> wall@. That diamond has a loop in every undirected sense --
-- first Betti number 1, a genuinely non-contractible circle -- and it
-- still runs down, because matter only ever goes one way round it.
-- Direction is not topological data, and direction is exactly what
-- decides whether a world sustains. Which is the same point 'flowOf'
-- already rests on: a break in a flow (a coupure) has an input side and
-- an output side, and they are not interchangeable.
--
-- So the invariant is topological, on the right object: a NON-TRIVIAL
-- STRONGLY CONNECTED COMPONENT of the flow digraph. Within such a
-- component every substance reaches every other and comes back, so the
-- cycle rank @E - V + 1@ of that component counts its INDEPENDENT
-- circuits -- how many different ways matter can go round, and therefore
-- how many edges you would have to cut before it stops circulating.
-- Rank 1 is a single loop that one broken machine ends. Rank 3 is a
-- world with somewhere else for matter to go.
--
-- The third verdict is the one that is NOT topological at all, and
-- saying so sharpens it: a SOURCE sustains a world without any circuit.
-- Rain is not a loop, it is an opening -- the flow graph coupled to
-- something outside itself. Topologically a source is just a leaf. What
-- makes it sustain is that the system is open, which is a boundary
-- condition, not a shape.
--
-- Read back onto the connective operators, they are exactly the moves
-- available on a digraph: @feed@ closes a circuit (raises the rank),
-- @bridge@ does the same for reachability, @replenish@ opens the
-- boundary instead, and @vista@ adds an edge carrying no flow at all --
-- a circuit in the reference graph and none in this one.

-- | Strongly connected components with more than one member (or a
-- self-loop): the parts of the flow graph where matter can actually go
-- round. Mutual reachability, computed directly -- these graphs have a
-- handful of nodes and clarity is worth more here than Tarjan.
circulating :: [(Text, Text)] -> [[Text]]
circulating edges =
  nub
    [ sort [b | b <- flowNodes edges, b `elem` reachFrom edges a, a `elem` reachFrom edges b]
    | a <- flowNodes edges
    , a `elem` reachFrom edges a
    ]

-- | Independent circuits in one component: @E - V + 1@.
cycleRank :: [(Text, Text)] -> [Text] -> Int
cycleRank edges comp =
  length [() | (a, b) <- edges, a `elem` comp, b `elem` comp] - length comp + 1

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
      putStrLn ("bred lineage (the TREE question -- parent A = newest offspring, B cycles seeds, mode cycles), " <> show (length rows) <> " generation(s):")
      mapM_
        (\(i, how, _, f, _) -> putStrLn ("  g" <> show i <> "  " <> T.unpack how <> fert f))
        rows
      putStrLn ""
      let (edges, sources, subs) = flowGraph seeds
          sinks = [x | x <- subs, x `notElem` map fst edges, x `notElem` sources]
          loops = cyclic edges
      putStrLn "substance flow (the rhizome question -- a DAG runs down, a cycle sustains):"
      if null subs
        then putStrLn "  none. No transition moves a unit, so there is no flow layer at all --\n  this is a pure reachability world and only the lineage above applies."
        else do
          mapM_ (\(a, b) -> putStrLn ("  " <> T.unpack a <> "  ->  " <> T.unpack b)) edges
          putStrLn ("  sources (produced consuming nothing): " <> showList' sources)
          putStrLn ("  sinks (consumed, never produced):     " <> showList' sinks)
          let comps = circulating edges
              ranks = [cycleRank edges c | c <- comps]
          putStrLn $
            if not (null loops)
              then
                "  CIRCULATES: "
                  <> show (length comps)
                  <> " strongly connected component(s), independent circuits "
                  <> show (sum ranks)
                  <> " -- cut that many flow edges and it stops.\n"
                  <> concat ["    {" <> T.unpack (T.intercalate ", " c) <> "}  rank " <> show r <> "\n" | (c, r) <- zip comps ranks]
                  <> "  This world SUSTAINS, and sustains ITSELF -- the circuit is closed, so nothing\n"
                  <> "  outside it is needed."
              else
                if null sources
                  then
                    "  every strongly connected component is trivial and there is no source: the flow\n"
                      <> "  graph is a partial order and every substance is a finite stock. This world RUNS DOWN."
                  else
                    "  no circuit, but "
                      <> showList' sources
                      <> " is produced from nothing -- sustained by a SOURCE.\n"
                      <> "  Not the same property as a cycle and worth not conflating: a source is not a\n"
                      <> "  shape in this graph at all, it is an opening. The world is sustained because it\n"
                      <> "  is OPEN, not because it circulates, and it stops the moment the outside does."
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
    showList' [] = "(none)"
    showList' xs = T.unpack (T.intercalate ", " xs)

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

    -- Pair each arg with the one BEFORE it, not the one after. The
    -- original paired forward and then ignored the pair's second
    -- element, so `--generations` was dropped and its VALUE was not --
    -- `--generations 8` fed "8" through as a seed path and died on
    -- openFile. Found by using the flag (2026-09-18); it had never
    -- worked, and the default 40 is rarely reached anyway because the
    -- walk stops at the first repeated shape.
    parseArgs as = (mapMaybe keep (zip ("" : as) as), gensOf as)
      where
        keep (prev, a)
          | take 2 a == "--" = Nothing
          | take 2 prev == "--" = Nothing
          | otherwise = Just a
        gensOf ("--generations" : n : _) = read n
        gensOf (_ : more) = gensOf more
        gensOf [] = 40

    load path = do
      src <- TIO.readFile path
      case parseMachineSurface src of
        Right m -> pure m
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure

