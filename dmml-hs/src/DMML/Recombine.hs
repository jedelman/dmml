{-# LANGUAGE OverloadedStrings #-}

-- | Recombination at AUTHORING time: cross two parent 'MachineStmt's
-- into a structurally novel offspring.
--
-- This is the other half of the recombination story
-- 'DMML.Ast.EffectGraft' opened. A graft is recombination at FIRE time
-- and it is coarse by construction: it copies a whole source machine
-- onto a target node, so grafting A then B onto one node gives you the
-- UNION of both parents' facts and nothing finer. That union is real
-- and useful, but it can only ever produce architecture one of the
-- parents already had, side by side. Nothing in a graft can build a
-- transition that requires what A required and yields what B yielded --
-- a shape neither parent has.
--
-- That finer crossover is what lives here. Because this module only
-- ever maps 'DMML.Ast' values to 'DMML.Ast' values, every offspring is
-- a well-typed 'MachineStmt' by construction and therefore renders to
-- grammatical DMML for free, exactly the guarantee
-- @app\/Cannon.hs@ already relies on. Semantic soundness is still the
-- guard's job downstream; what this module owes is STRUCTURAL
-- soundness, and there is exactly one invariant that costs real work to
-- keep:
--
-- == The state-alignment problem
--
-- A machine has ONE current state. Parent A's lifecycle might be
-- @sealed -> open@ and parent B's @unchosen -> chosen@. Naively union
-- the state lists and you get a four-state machine with two disjoint
-- lifecycles -- which is not a crossover at all, it is a corpse: once
-- A's transition fires the machine sits in @open@, and every one of B's
-- transitions is dead forever because nothing can ever put it in
-- @unchosen@. The offspring would parse, type-check, and never work.
--
-- So B's state space is ALIGNED onto A's, positionally: B's i-th state
-- is renamed to A's i-th state, and every copied B transition's
-- @from@\/@to@ is rewritten through that map. The offspring's lifecycle
-- IS A's lifecycle; what it inherits from B is B's relational content
-- (its guards, its effects, its onward frontier), now living inside a
-- lifecycle that can actually run. Surplus B states (when B has more
-- states than A) are appended under their own idents, renamed on
-- collision -- so a longer-lived B extends A's lifecycle onward rather
-- than forking it.
--
-- Positional alignment is a CHOICE, and a disclosed one: it assumes a
-- machine's states are written in lifecycle order, which is the
-- convention every machine in this project actually follows (@sealed@
-- before @open@, @unchosen@ before @chosen@). It is not derived from
-- the transition graph. A parent whose states are declared out of
-- lifecycle order will breed a coherent but differently-shaped
-- offspring than its author expected.
--
-- == What each crossover mode means
--
-- ['Union'] Every transition of A, then every remapped transition of B.
--   The authoring-time equivalent of a double graft -- same coarse
--   union, but resolved before the machine is ever committed, so
--   collisions are handled by renaming instead of by silent overwrite.
--
--   Worth being precise about what a union offspring IS, because the
--   obvious reading is wrong: because state alignment rebases B's
--   transitions onto A's single lifecycle, a union of two two-state
--   parents is a MENU, not an accumulation. All of its transitions
--   leave the same state and arrive at the same one, so exactly one of
--   them will ever fire and the rest are foreclosed the moment it does.
--   The offspring is a room that could be a vault OR a fork OR a forge,
--   and firing decides which -- which is the same permanent, real
--   choice @app\/Cannon.hs@'s fork variant exists to create, arriving
--   here by a different route. A parent with a longer lifecycle is what
--   turns a menu back into a sequence.
--
-- ['Splice'] Single-point crossover on the transition list read as a
--   chromosome: the first @k@ transitions from A, the rest from B. The
--   offspring is a real mosaic -- part of one parent's behaviour, part
--   of the other's -- and varying @k@ gives a whole family of siblings
--   from one pair.
--
-- ['Chimera'] Crossover INSIDE a transition, at the one seam a
--   transition actually has: a transition says what must hold
--   (guards) and what follows (effects). A chimeric transition keeps
--   A's identity, parameters, lifecycle and guards, and takes B's
--   WORLD effects. A vault's key requirement with a fork's
--   path-opening yield is an architectural element neither parent
--   had and no graft could produce.
--
--   The lifecycle effects (@assert self \`state\` X@ and @retract self
--   \`state\`@) are deliberately NOT crossed: they are taken from A,
--   whose @from@\/@to@ the offspring transition keeps. Crossing them
--   too would be the state-alignment bug in miniature -- B's
--   @assert self \`state\` chosen@ remaps to whichever A-state sits at
--   @chosen@'s index, which need not be the state A's transition
--   actually travels to, leaving a transition that asserts one state
--   and moves to another. The rule that avoids it is crisp: the
--   lifecycle belongs to A, the consequences come from B.
--
-- == Recombination cannot preserve cross-machine exclusivity
--
-- Found in a real self-extending run (2026-09-18), not by reasoning:
-- a delve took the WEST branch of @app\/Cannon.hs@'s fork, which is
-- supposed to seal the east permanently, and eight generations later
-- was breaching eastern rooms anyway.
--
-- The mechanism is exact, and it is a property of the language rather
-- than a defect here. A fork's two transitions are mutually exclusive
-- because they SHARE ONE @unchosen -> chosen@ lock on ONE machine;
-- firing either moves that machine, and the other can never fire. That
-- exclusivity is not a property of the transitions, it is a property of
-- the machine they sit in. Breed one of them onto a different machine
-- and it arrives with its EFFECT (@assert path\/east \`cleared\`
-- mark\/yes@) but under a different lock -- so an offspring can hand
-- out a yield the parent had permanently foreclosed.
--
-- Generalized: **any invariant this language enforces through a shared
-- lifecycle lock is breakable by breeding a transition out of the
-- machine that holds it.** Exclusivity is per-machine; crossover moves
-- transitions between machines; the two compose exactly as badly as
-- that sentence implies.
--
-- Deliberately NOT prevented. A room that reopens a foreclosed path is
-- precisely the architecture neither parent had, which is what this
-- module exists to produce, and the alternative -- refusing to copy any
-- transition whose lock it shares with a sibling -- would forbid
-- breeding forks at all, the richest parents in the corpus. Whoever
-- wants the guarantee back should reach for a guard that encodes the
-- foreclosure as a FACT (a @path\/east \`sealed\` mark\/yes@ the
-- offspring must also test), because a fact survives recombination and
-- a lifecycle lock does not.
--
-- == Node re-derivation
--
-- A generated room bakes its own identity into the literals it asserts
-- (@assert self \`yields\` sigil\/wVault@ in a machine named
-- @room\/wVault@). Copied verbatim into an offspring, that literal
-- still names the PARENT's sigil -- the offspring would yield its
-- parent's prize. So every node literal in a copied transition has any
-- segment (or dot-piece of a segment) equal to a parent's own last
-- segment replaced with the offspring's last segment. It is the same
-- rule 'DMML.MachineFacts.encodeMachine' uses to derive sub-nodes from
-- a machine node, applied in reverse.
--
-- Disclosed cost: this is a NAME-based rule, so it will also rewrite an
-- unrelated node that happens to share a parent's last segment. In this
-- project's naming that coincidence means the nodes really are related
-- (@sigil\/wVault@ IS wVault's sigil), but nothing enforces that, and a
-- corpus that reuses last segments across unrelated namespaces would
-- see false rewrites.
module DMML.Recombine
  ( Crossover (..)
  , RecombineError (..)
  , breed
  , breedPool
  , reanchor
  , crossoverLabel
  ) where

import Data.List (nub)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

-- | How two parents' transition lists are combined. See the module
-- haddock for what each one means architecturally.
data Crossover
  = Union
  | -- | Single-point crossover: @Splice k@ takes the first @k@
    -- transitions from A and everything from index @k@ onward from B.
    Splice Int
  | Chimera
  deriving (Eq, Show)

data RecombineError
  = -- | A transition names a @from@\/@to@ state the offspring does not
    -- declare. The whole point of state alignment is that this is
    -- unreachable; it is checked anyway, because an offspring that
    -- violates it is exactly the dead machine this module exists to
    -- avoid, and a silent one is worse than a refused one.
    UndeclaredState Text Text
  | -- | @Splice k@ with @k@ outside @[0, length (transitions A)]@.
    SpliceOutOfRange Int Int
  | -- | Neither parent has a transition, so there is nothing to cross.
    NothingToCross
  deriving (Eq, Show)

-- | A short, stable, filename-safe tag for a crossover mode -- used to
-- name the candidates in a bred pool.
crossoverLabel :: Crossover -> Text
crossoverLabel Union = "union"
crossoverLabel Chimera = "chimera"
crossoverLabel (Splice k) = "splice" <> T.pack (show k)

-- Node re-derivation -------------------------------------------------------

lastSeg :: NodeRef -> Text
lastSeg (NodeRef segs) = case reverse segs of
  (s : _) -> last (T.splitOn "." s)
  [] -> ""

-- | Replace any segment or dot-piece equal to @from@ with @to@. Splits
-- on @\/@ then @.@ to match 'DMML.FromJson.isValidNodeRef''s own reading
-- of a node ref's shape.
rederiveText :: Text -> Text -> Text -> Text
rederiveText from to =
  T.intercalate "/" . map perSegment . T.splitOn "/"
  where
    perSegment = T.intercalate "." . map swap . T.splitOn "."
    swap piece
      | piece == from = to
      | otherwise = piece

rederiveNodeRef :: Text -> Text -> NodeRef -> NodeRef
rederiveNodeRef from to (NodeRef segs) =
  NodeRef (map (T.intercalate "." . map swap . T.splitOn ".") segs)
  where
    swap piece
      | piece == from = to
      | otherwise = piece

-- Generic term mapping -----------------------------------------------------

-- | Apply a rewrite to every LITERAL node mentioned anywhere in a
-- transition -- guard anchors, guard hop terms, effect subjects, effect
-- values, spawn templates, graft sources. @self@, @$param@ and pattern
-- variables are untouched: they are not names of anything in the world,
-- they resolve at fire time.
--
-- Lifecycle effects are skipped entirely. The @open@ in
-- @assert self \`state\` open@ is stored as a 'TermNode', but it names a
-- STATE, not a node in the world -- re-deriving it would be a category
-- error, and one that silently breaks the machine's own lifecycle.
-- 'remapLifecycle' is what rewrites those, and only through the state
-- alignment map.
mapNodeLiterals :: (Text -> Text) -> (NodeRef -> NodeRef) -> TransitionDecl -> TransitionDecl
mapNodeLiterals f fRef t =
  t
    { transitionGuards = map onGuard (transitionGuards t)
    , transitionEffects = map onEffect (transitionEffects t)
    }
  where
    onTerm (TermNode n) = TermNode (f n)
    onTerm other = other

    onHop h = h {hopTerm = onTerm (hopTerm h)}

    onGuard g =
      let e = guardExists g
          p = existsPattern e
       in g
            { guardExists =
                e
                  { existsPattern =
                      p
                        { patternAnchor = onTerm (patternAnchor p)
                        , patternHops = map onHop (patternHops p)
                        }
                  }
            }

    onValue (EffectValueTerm term) = EffectValueTerm (onTerm term)
    onValue lit@(EffectValueLiteral _) = lit

    onEffect e | isLifecycleEffect e = e
    onEffect (EffectAssert subj p v) = EffectAssert (onTerm subj) p (onValue v)
    onEffect (EffectRetract subj hops p mv) =
      EffectRetract (onTerm subj) (map onHop hops) p (fmap onValue mv)
    onEffect (EffectSpawn newNode template) = EffectSpawn (onTerm newNode) (fRef template)
    onEffect (EffectGraft target source) = EffectGraft (onTerm target) (onTerm source)

-- | Normalize every span in a transition. An offspring is NEW: it was
-- authored here, not at whatever pointer its parents' spans recorded,
-- and carrying a parent's provenance into a child would be a small lie
-- in every error message the offspring ever produces. Normalizing also
-- makes structurally identical offspring compare equal, which is what
-- lets 'breedPool' deduplicate.
respan :: Span -> TransitionDecl -> TransitionDecl
respan sp t =
  t
    { transitionSpan = sp
    , transitionGuards = map onGuard (transitionGuards t)
    }
  where
    onGuard g = g {guardSpan = sp, guardExists = (guardExists g) {existsSpan = sp}}

-- State alignment ----------------------------------------------------------

-- | Pick an ident not already in use, by suffixing @_2@, @_3@, ... The
-- suffix keeps the result a valid ident under
-- 'DMML.FromJson.isValidIdent' (@[A-Za-z][A-Za-z0-9_]*@) for any valid
-- input.
freshIdent :: [Text] -> Text -> Text
freshIdent taken wanted
  | wanted `notElem` taken = wanted
  | otherwise = go (2 :: Int)
  where
    go n =
      let candidate = wanted <> "_" <> T.pack (show n)
       in if candidate `elem` taken then go (n + 1) else candidate

-- | Offspring state list, and the rename map carrying B's states onto
-- it. B's i-th state takes A's i-th name; B's surplus states keep their
-- own idents (renamed on collision) and extend the list.
alignStates :: MachineStmt -> MachineStmt -> ([StateDecl], Map Text Text)
alignStates a b = (machineStates a ++ reverse surplusDecls, Map.fromList (paired ++ surplusPairs))
  where
    aNames = map stateIdent (machineStates a)
    bNames = map stateIdent (machineStates b)
    paired = zip bNames aNames
    surplus = drop (length aNames) bNames
    (surplusDecls, surplusPairs, _) = foldl step ([], [], aNames) surplus
    step (decls, pairs, taken) bName =
      let fresh = freshIdent taken bName
       in ( StateDecl fresh (machineSpan a) : decls
          , pairs ++ [(bName, fresh)]
          , taken ++ [fresh]
          )

-- | Rewrite a B transition's lifecycle through the alignment map.
--
-- A transition names its states in TWO places, and both must move
-- together: the @from -> to@ line, and the lifecycle effect
-- @assert self \`state\` X@ that records the arrival as a fact. Rewriting
-- only the first produces a transition that travels to @open@ while
-- asserting @state chosen@ -- a state the offspring does not even
-- declare. It parses, it renders, and the first time it fires it puts
-- the machine into a state nothing can transition out of. That is the
-- same dead-machine failure naive state unioning causes, arriving
-- through a different door.
--
-- A state B declares but the map somehow misses is left alone, and the
-- final validation in 'breed' turns that into a refusal rather than a
-- dead offspring.
remapLifecycle :: Map Text Text -> TransitionDecl -> TransitionDecl
remapLifecycle m t =
  t
    { transitionFrom = fmap look (transitionFrom t)
    , transitionTo = fmap look (transitionTo t)
    , transitionEffects = map onEffect (transitionEffects t)
    }
  where
    look s = Map.findWithDefault s s m
    onEffect (EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode s))) =
      EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode (look s)))
    onEffect e = e

-- Lifecycle effects --------------------------------------------------------

-- | An effect that moves the machine through its own state machine,
-- rather than saying anything about the world. These are the effects a
-- chimera must NOT cross (see module haddock).
isLifecycleEffect :: Effect -> Bool
isLifecycleEffect (EffectAssert TermSelf (PredIdent "state") _) = True
isLifecycleEffect (EffectRetract TermSelf _ (PredIdent "state") _) = True
isLifecycleEffect _ = False

-- Breeding -----------------------------------------------------------------

-- | Cross two parents into one offspring.
--
-- Every span in the result is @sp@ -- the offspring's provenance is the
-- act of breeding, not either parent's source.
breed ::
  Span ->
  Crossover ->
  -- | The offspring's own node.
  NodeRef ->
  -- | Parent A, whose lifecycle and state space the offspring inherits.
  MachineStmt ->
  -- | Parent B, aligned onto A.
  MachineStmt ->
  Either RecombineError MachineStmt
breed sp mode child a b
  | null (machineTransitions a) && null (machineTransitions b) = Left NothingToCross
  | Splice k <- mode
  , k < 0 || k > length (machineTransitions a) =
      Left (SpliceOutOfRange k (length (machineTransitions a)))
  | otherwise = validate offspring
  where
    childLast = lastSeg child
    (states, stateMap) = alignStates a b

    -- Both parents' self-derived literals are re-pointed at the child,
    -- symmetrically -- neither parent is privileged here, only in the
    -- lifecycle.
    rederiveFrom p =
      let pl = lastSeg (machineNode p)
       in respan sp . mapNodeLiterals (rederiveText pl childLast) (rederiveNodeRef pl childLast)

    aTs = map (rederiveFrom a) (machineTransitions a)
    -- Align B's lifecycle first, then re-derive its world literals: the
    -- two rewrites touch disjoint parts of a transition by construction
    -- (see 'mapNodeLiterals'), but doing the alignment against B's own
    -- original state idents keeps that independence obvious.
    bTs = map (rederiveFrom b . remapLifecycle stateMap) (machineTransitions b)

    combined = case mode of
      Union -> aTs ++ bTs
      Splice k -> take k aTs ++ drop k bTs
      Chimera ->
        zipWith chimeraOf aTs bTs
          ++ drop (length bTs) aTs
          ++ drop (length aTs) bTs

    -- A's identity, parameters, lifecycle, guards and state-boilerplate;
    -- B's consequences. World effects lead, lifecycle effects trail --
    -- the same order every hand-written and cannon-fired machine in this
    -- project already renders in.
    chimeraOf ta tb =
      ta
        { transitionEffects =
            filter (not . isLifecycleEffect) (transitionEffects tb)
              ++ filter isLifecycleEffect (transitionEffects ta)
        }

    offspring =
      MachineStmt
        { machineNode = child
        , machineStates = map (\s -> s {stateSpan = sp}) states
        , machineTransitions = map dedup (renameCollisions combined)
        , machineSpan = sp
        }

    -- Union and chimera routinely land the same guard twice (both
    -- parents gated on the same node) or the same effect twice. Keep
    -- first occurrence, drop the rest.
    dedup t =
      t
        { transitionGuards = nub (transitionGuards t)
        , transitionEffects = nub (transitionEffects t)
        }

    validate m =
      let declared = map stateIdent (machineStates m)
          bad =
            [ (transitionIdent t, s)
            | t <- machineTransitions m
            , s <- [x | Just x <- [transitionFrom t]] ++ [x | Just x <- [transitionTo t]]
            , s `notElem` declared
            ]
       in case bad of
            ((tid, s) : _) -> Left (UndeclaredState tid s)
            [] -> Right m

-- | Make transition idents unique, in order, first occurrence winning.
-- This is the "handle collisions by renaming at authoring time" half:
-- two parents that both call their transition @breach@ give an
-- offspring with @breach@ and @breach_2@, and both remain fireable,
-- where a fire-time graft would just have one silently shadow the other.
renameCollisions :: [TransitionDecl] -> [TransitionDecl]
renameCollisions = go []
  where
    go _ [] = []
    go taken (t : ts) =
      let fresh = freshIdent taken (transitionIdent t)
       in t {transitionIdent = fresh} : go (taken ++ [fresh]) ts

-- | Re-root an offspring onto a new parent node in the dungeon.
--
-- A bred offspring inherits BOTH parents' literal reachability guards,
-- so by default it sits at a confluence: it only opens where both
-- lineages have been cleared. That is a real architectural shape and
-- sometimes the wanted one, but a breeder that can only ever produce
-- confluences cannot extend a frontier, so this rebinds them.
--
-- It rewrites guards of exactly the shape @\<node\> \`cleared\`
-- mark\/yes@ -- the reachability convention @app\/Cannon.hs@ emits for
-- every room and fork it fires. That is a CONVENTION, not a property of
-- the grammar: a machine that expresses reachability some other way is
-- untouched by this, and a machine that uses @cleared@ to mean
-- something else will be rewritten wrongly. Duplicate guards created by
-- collapsing several anchors into one are removed.
reanchor :: Text -> MachineStmt -> MachineStmt
reanchor anchor m =
  m {machineTransitions = map onTransition (machineTransitions m)}
  where
    onTransition t = t {transitionGuards = nub (map onGuard (transitionGuards t))}
    onGuard g =
      let e = guardExists g
          p = existsPattern e
       in case (guardNegated g, patternAnchor p, patternHops p) of
            (False, TermNode _, [PatternHop "cleared" (TermNode "mark/yes")]) ->
              g {guardExists = e {existsPattern = p {patternAnchor = TermNode anchor}}}
            _ -> g

-- | Fire a whole FRONTIER of offspring from one parent pair, rather
-- than a single shot -- every crossover mode, in both orientations,
-- deduplicated structurally.
--
-- Orientation matters and is not symmetric: @breed Chimera child A B@
-- keeps A's guards and takes B's effects, while swapping the parents
-- keeps B's guards and takes A's. Those are different architectures, so
-- both are candidates. Structurally identical results (common when the
-- parents are similar) collapse to one, which is why every span is
-- normalized first.
--
-- Each candidate carries its own suffixed node, so the pool is directly
-- committable: @child@ @room\/x@ yields @room\/x_union@,
-- @room\/x_chimera@, @room\/x_splice1@, and their @_rev@ twins.
breedPool ::
  Span ->
  -- | Node prefix; each candidate appends its own label to the last segment.
  NodeRef ->
  MachineStmt ->
  MachineStmt ->
  [(Text, MachineStmt)]
breedPool sp prefix a b = dedupOn shape
  [ (label, m)
  | (suffix, x, y) <- [("", a, b), ("_rev", b, a)]
  , mode <- modesFor x y
  , let label = crossoverLabel mode <> suffix
  , Right m <- [breed sp mode (suffixNode label) x y]
  ]
  where
    -- Compare offspring with their distinguishing node erased, so two
    -- modes that happen to produce the same architecture collapse to
    -- one candidate instead of two differently-named copies.
    shape (_, m) = m {machineNode = prefix}

    -- Only splice points where BOTH parents actually contribute: A
    -- gives @take k@ (needs k >= 1) and B gives @drop k@ (needs
    -- k < length B). Outside that range a "crossover" is just one
    -- parent, truncated.
    modesFor x y =
      [Union, Chimera]
        ++ [ Splice k
           | k <- [1 .. min (length (machineTransitions x)) (length (machineTransitions y) - 1)]
           ]

    suffixNode label = case reverse (nodeRefSegments prefix) of
      (s : rest) -> NodeRef (reverse rest ++ [s <> "_" <> label])
      [] -> NodeRef [label]

    dedupOn f = go []
      where
        go _ [] = []
        go seen (x : xs)
          | f x `elem` seen = go seen xs
          | otherwise = x : go (f x : seen) xs
