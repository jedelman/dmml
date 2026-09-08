{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ForeignFunctionInterface #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TypeApplications #-}

-- | The real Android JNI bridge surface -- follows dmml/dev-journal/
-- 2026-09-04-android-jni-vs-ipc.md's recommendation to the letter: JNI,
-- in-process, calling dmml-hs's OWN library functions directly
-- ('DMML.Materialize.applyCommit'\/'renderSnapshot', 'DMML.Guard.
-- availableTransitions', 'DMML.Fire.fireTransition'\/'renderFiredCommit')
-- -- the same layer @app/RenderSnapshot.hs@ and @app/FireTransition.hs@
-- already sit at -- rather than a second wrapper around either CLI
-- executable's argv interface.
--
-- The original v1 surface (@dmml_render@\/@dmml_actions@\/@dmml_fire@)
-- is narrower than the CLIs' own multi-file generality: exactly ONE
-- world commit and ONE machine, mirroring @examples/world-browser-demo@'s
-- own fixture shape rather than @app/RenderSnapshot.hs@'s N-file list.
-- That was an honest v1 scoping, not a design ceiling -- and it turned
-- out to be a REAL LIMIT the moment a caller needed more than one fire:
-- a Compose UI (or any multi-round client) needs to see the state AFTER
-- firing, then fire again against THAT state, the same accumulation
-- 'app/TouchBrowser.hs'\/'app/InteractiveBrowser.hs' already do natively
-- against 'DMML.Materialize.applyIdentifiedCommits' over a growing
-- @[IdentifiedCommit]@ -- but the original single-commit functions have
-- no way to accept that growing history across the FFI boundary.
--
-- Fixed here with the @_history@ variants below, encoding the history as
-- a JSON array of strings (@Data.Aeson@, already a dependency) rather
-- than inventing a raw @jobjectArray@\/@Ptr (Ptr CChar)@ marshaling
-- scheme -- a real, working array-passing mechanism, chosen specifically
-- because it reuses a library already proven correct instead of hand-
-- rolling one. The original single-commit functions are kept, unchanged
-- (a single-shot caller with no history to track still doesn't need
-- one).
--
-- Every exported function wraps its entire body in 'Control.Exception.try'
-- and marshals ANY failure (a parse error, a 'FireError', or a genuinely
-- unexpected exception) back as a plain @\"ERROR: ...\"@-prefixed
-- 'CString' -- never lets an exception escape across the FFI boundary
-- raw, exactly the mitigation the JNI-vs-IPC dev-journal entry calls
-- \"required for the real bridge\" (no process isolation on this path,
-- unlike a subprocess's own clean nonzero exit).
--
-- A THIRD family, @_dir@, added 2026-09-07 per @dev-journal/2026-09-07-
-- android-canonical-structure-decisions.md@: the @_history@ family's
-- JSON-array-of-strings shape was the right call to prove the FFI
-- surface for F1, but the wrong long-term shape once a real @commits/@
-- directory exists on-device (JGit-managed, per that same entry) --
-- every render would otherwise need Kotlin to re-serialize the whole
-- history as JSON on every call, duplicating what 'DMML.Loader' now
-- does correctly (real per-file provenance, real mtime ordering) in a
-- second, Kotlin-side implementation. The @_dir@ functions instead take
-- a real filesystem path and call 'DMML.Loader.worldSnapshotFromDirectory'
-- directly -- and, as a direct consequence of loading a whole directory
-- rather than one passed-in machine string, are MULTI-MACHINE: every
-- @.dmml@ machine file present loads, not just one. 'dmml_fire_dir'
-- therefore takes an explicit machine-node argument (which machine, of
-- however many are present, is actually firing) that the single-machine
-- @_history@\/original functions never needed. All three prior families
-- are kept, unchanged -- a caller with in-memory content and no real
-- directory (the host smoke test, e.g.) still has a legitimate use for
-- them.
module DMML.JniBridge
  ( dmml_render
  , dmml_actions
  , dmml_fire
  , dmml_render_history
  , dmml_actions_history
  , dmml_fire_history
  , dmml_render_dir
  , dmml_actions_dir
  , dmml_fire_dir
  , renderBridge
  , actionsBridge
  , fireBridge
  , renderHistoryBridge
  , actionsHistoryBridge
  , fireHistoryBridge
  , renderDirBridge
  , actionsDirBridge
  , fireDirBridge
  ) where

import Control.Exception (SomeException, displayException, try)
import Data.Aeson (decode)
import qualified Data.ByteString.Lazy as BL
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Foreign.C.String (CString, newCString, peekCString)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt, NodeRef (..), machineNode, nodeRefSegments)
import DMML.Fire (FireError, fireTransition, renderFiredCommit)
import DMML.Guard (EvalContext (..), availableTransitions)
import DMML.LocalIdentity (localFileRef)
import DMML.Loader (worldSnapshotFromDirectory)
import DMML.Materialize (IdentifiedCommit (..), WorldSnapshot, applyIdentifiedCommit, applyIdentifiedCommits, emptySnapshot, renderSnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

foreign export ccall dmml_render :: CString -> CString -> IO CString
foreign export ccall dmml_actions :: CString -> CString -> CString -> IO CString
foreign export ccall dmml_fire :: CString -> CString -> CString -> CString -> IO CString
foreign export ccall dmml_render_history :: CString -> CString -> IO CString
foreign export ccall dmml_actions_history :: CString -> CString -> CString -> IO CString
foreign export ccall dmml_fire_history :: CString -> CString -> CString -> CString -> IO CString
foreign export ccall dmml_render_dir :: CString -> IO CString
foreign export ccall dmml_actions_dir :: CString -> CString -> IO CString
foreign export ccall dmml_fire_dir :: CString -> CString -> CString -> CString -> IO CString

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | Parses the one world commit and the one machine (an empty\/blank
-- machine source is legal -- "no machine equipped," not an error) and
-- materializes the commit WITH real content-addressed provenance
-- ('DMML.LocalIdentity.localFileRef' over the caller-supplied source
-- text itself, labeled @\"android:worldSrc\"@ since there is no real
-- file path on this side of the JNI boundary -- 'localFileRef' only
-- ever uses its path argument as an opaque label, never reads the
-- filesystem, so this is legitimate, not a workaround). This is
-- load-bearing, not incidental: exactly like @app/FireTransition.hs@'s
-- own @--world@ handling, 'DMML.Fire.fireTransition' refuses to fire
-- ANY transition with a @retract@ effect (i.e. almost any real state
-- transition) against a snapshot materialized without real provenance
-- ('DMML.Fire.FireRetractNoProvenance') -- using the plain,
-- provenance-free 'DMML.Materialize.applyCommit' here would silently
-- make 'fireBridge' unable to fire the one kind of transition a real
-- machine actually has.
materialize :: Text -> Text -> Either String (WorldSnapshot, Maybe (Text, MachineStmt))
materialize worldSrc machineSrc = do
  worldStmt <- either (Left . errorBundlePretty) Right (parseCommitSurface worldSrc)
  machine <-
    if T.all (`elem` (" \t\r\n" :: String)) machineSrc
      then Right Nothing
      else case parseMachineSurface machineSrc of
        Right m -> Right (Just (nodeRefText (machineNode m), m))
        Left err -> Left (errorBundlePretty err)
  let ref = localFileRef "android:worldSrc" (TE.encodeUtf8 worldSrc)
      identified = IdentifiedCommit {icRef = ref, icCommit = worldStmt}
  pure (applyIdentifiedCommit "world" identified emptySnapshot, machine)

-- | Pure core of 'dmml_render': materialize, then render -- exactly
-- @app/RenderSnapshot.hs@'s own two-step pipeline, minus its N-file
-- loop and governance pass (no divergent machine set to arbitrate
-- between when there is only ever at most one machine here).
renderBridge :: Text -> Text -> Either String Text
renderBridge worldSrc machineSrc = do
  (snap, _machine) <- materialize worldSrc machineSrc
  pure (renderSnapshot snap)

-- | Pure core of 'dmml_actions': materialize, then enumerate every
-- transition on the one supplied machine (if any) that currently holds
-- for @selfNode@ -- 'DMML.Guard.availableTransitions' unchanged, called
-- over a one-entry machine map, same primitive
-- @examples/world-browser-demo@ already proved. Rendered as
-- newline-separated @machineNode\/transitionIdent@ pairs; @\"\"@ (not an
-- error) when nothing is currently legal.
actionsBridge :: Text -> Text -> Text -> Either String Text
actionsBridge worldSrc machineSrc selfNode = do
  (snap, machine) <- materialize worldSrc machineSrc
  let machineMap = maybe Map.empty (\(k, m) -> Map.singleton k m) machine
      ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
      actions = availableTransitions machineMap ctx snap
  pure (T.unlines [m <> "/" <> t | (m, t) <- actions])

-- | Pure core of 'dmml_fire': materialize, then fire one named
-- transition via 'DMML.Fire.fireTransition' against the single supplied
-- machine (both as the firing machine and as the whole known machine
-- set gated against -- a real, disclosed narrowing: a world with
-- several live machines could have a firing that looks consistency-safe
-- here but isn't once every OTHER machine is also in scope; the multi-
-- machine gate is exactly as real as the caller's machine set, same
-- caveat 'DMML.Fire.fireTransition'\'s own doc comment already states).
-- On success, renders the fired effects as a real, re-parseable DMML
-- Surface commit ('DMML.Fire.renderFiredCommit') -- the caller applies
-- it (or not) exactly like any other commit, this function never
-- mutates anything itself, per 'DMML.Fire'\'s own "DMML is the
-- evidence, not any tool's say-so" discipline.
fireBridge :: Text -> Text -> Text -> Text -> Either String Text
fireBridge worldSrc machineSrc selfNode transitionIdent = do
  (snap, machine) <- materialize worldSrc machineSrc
  case machine of
    Nothing -> Left "no machine supplied to fire a transition against"
    Just (machineKey, m) -> do
      let machineMap = Map.singleton machineKey m
          ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
      case fireTransition machineMap m transitionIdent ctx snap of
        Left err -> Left (renderFireError err)
        Right effects -> pure (renderFiredCommit "android_fire" effects)

renderFireError :: FireError -> String
renderFireError = show

-- | Parses a JSON array of world-commit source strings (oldest first --
-- the original world, then each fired commit in firing order, exactly
-- 'app/TouchBrowser.hs'\'s own @[IdentifiedCommit]@ history, just
-- serialized to cross the FFI boundary) and the one machine, giving
-- EACH history entry its own real content-addressed provenance
-- (labeled by its position, @\"android:history0\"@, @\"android:history1\"@,
-- ...) via 'DMML.LocalIdentity.localFileRef' -- so a retract in entry N
-- can cite entry N's own real fact, not a fabricated shared one.
-- Requires at least one history entry (the original world); an empty
-- array is a caller error, not silently treated as an empty world.
materializeHistory :: Text -> Text -> Either String (WorldSnapshot, Maybe (Text, MachineStmt))
materializeHistory historyJson machineSrc = do
  worldSrcs <- maybe (Left "history is not a JSON array of strings") Right (decode (BL.fromStrict (TE.encodeUtf8 historyJson)) :: Maybe [Text])
  case worldSrcs of
    [] -> Left "history must contain at least one world commit"
    _ -> do
      worldStmts <- traverse (either (Left . errorBundlePretty) Right . parseCommitSurface) worldSrcs
      machine <-
        if T.all (`elem` (" \t\r\n" :: String)) machineSrc
          then Right Nothing
          else case parseMachineSurface machineSrc of
            Right m -> Right (Just (nodeRefText (machineNode m), m))
            Left err -> Left (errorBundlePretty err)
      let identified =
            [ IdentifiedCommit {icRef = localFileRef ("android:history" <> show i) (TE.encodeUtf8 src), icCommit = stmt}
            | (i, src, stmt) <- zip3 [0 :: Int ..] worldSrcs worldStmts
            ]
      pure (applyIdentifiedCommits "world" identified, machine)

-- | History-aware counterpart of 'renderBridge'.
renderHistoryBridge :: Text -> Text -> Either String Text
renderHistoryBridge historyJson machineSrc = do
  (snap, _machine) <- materializeHistory historyJson machineSrc
  pure (renderSnapshot snap)

-- | History-aware counterpart of 'actionsBridge'.
actionsHistoryBridge :: Text -> Text -> Text -> Either String Text
actionsHistoryBridge historyJson machineSrc selfNode = do
  (snap, machine) <- materializeHistory historyJson machineSrc
  let machineMap = maybe Map.empty (\(k, m) -> Map.singleton k m) machine
      ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
      actions = availableTransitions machineMap ctx snap
  pure (T.unlines [m <> "/" <> t | (m, t) <- actions])

-- | History-aware counterpart of 'fireBridge'. Returns just the NEWLY
-- fired commit's text (as a one-element JSON-string-encoded result, same
-- shape as any other successful call here) -- the caller appends it to
-- its own history list and passes the extended list on the next call,
-- exactly the accumulation 'app/TouchBrowser.hs' already does natively;
-- this function never mutates or extends the history itself.
fireHistoryBridge :: Text -> Text -> Text -> Text -> Either String Text
fireHistoryBridge historyJson machineSrc selfNode transitionIdent = do
  (snap, machine) <- materializeHistory historyJson machineSrc
  case machine of
    Nothing -> Left "no machine supplied to fire a transition against"
    Just (machineKey, m) -> do
      let machineMap = Map.singleton machineKey m
          ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
      case fireTransition machineMap m transitionIdent ctx snap of
        Left err -> Left (renderFireError err)
        Right effects -> pure (renderFiredCommit "android_fire" effects)

-- | Pure core of 'dmml_render_dir': load every @*.dmml@ file directly
-- inside @dirPath@ (via 'DMML.Loader.worldSnapshotFromDirectory' --
-- real mtime ordering, real per-file provenance) and render the
-- resulting snapshot. Ignores whatever machines are present -- 'render'
-- never needed a machine, single- or multi-.
renderDirBridge :: Text -> IO (Either String Text)
renderDirBridge dirPath = do
  result <- worldSnapshotFromDirectory (T.unpack dirPath)
  pure $ case result of
    Left err -> Left err
    Right (snap, _machines) -> Right (renderSnapshot snap)

-- | Pure core of 'dmml_actions_dir': load the directory, then enumerate
-- every transition on EVERY machine found there that currently holds
-- for @selfNode@ -- 'DMML.Guard.availableTransitions' over the real,
-- possibly-multi-entry 'DMML.Loader.machineMap', not a caller-supplied
-- single machine.
actionsDirBridge :: Text -> Text -> IO (Either String Text)
actionsDirBridge dirPath selfNode = do
  result <- worldSnapshotFromDirectory (T.unpack dirPath)
  pure $ case result of
    Left err -> Left err
    Right (snap, machines) ->
      let ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
          actions = availableTransitions machines ctx snap
       in Right (T.unlines [m <> "/" <> t | (m, t) <- actions])

-- | Pure core of 'dmml_fire_dir': load the directory, then fire one
-- named transition on the EXPLICITLY named machine (there can be
-- several present, unlike the single-machine families above, so the
-- caller must say which one) -- gated against every OTHER machine found
-- in the same directory too, same "the multi-machine gate is exactly as
-- real as the caller's machine set" caveat 'fireBridge'\'s own doc
-- comment already states, just now genuinely populated by every machine
-- file present rather than at most one.
fireDirBridge :: Text -> Text -> Text -> Text -> IO (Either String Text)
fireDirBridge dirPath selfNode machineNodeText transitionIdent = do
  result <- worldSnapshotFromDirectory (T.unpack dirPath)
  pure $ case result of
    Left err -> Left err
    Right (snap, machines) -> case Map.lookup machineNodeText machines of
      Nothing -> Left ("no such machine in directory: " <> T.unpack machineNodeText)
      Just m ->
        let ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
         in case fireTransition machines m transitionIdent ctx snap of
              Left err -> Left (renderFireError err)
              Right effects -> Right (renderFiredCommit "android_fire" effects)

-- | Marshals a 'Left' to an @\"ERROR: ...\"@-prefixed 'CString' and a
-- 'Right' straight through -- shared by all three exported functions so
-- the error-shape discipline the dev-journal calls for lives in exactly
-- one place, not reimplemented three times.
marshal :: Either String Text -> IO CString
marshal (Left err) = newCString ("ERROR: " <> err)
marshal (Right ok) = newCString (T.unpack ok)

dmml_render :: CString -> CString -> IO CString
dmml_render worldC machineC = guardedRun $ do
  worldSrc <- T.pack <$> peekCString worldC
  machineSrc <- T.pack <$> peekCString machineC
  marshal (renderBridge worldSrc machineSrc)

dmml_actions :: CString -> CString -> CString -> IO CString
dmml_actions worldC machineC selfC = guardedRun $ do
  worldSrc <- T.pack <$> peekCString worldC
  machineSrc <- T.pack <$> peekCString machineC
  selfNode <- T.pack <$> peekCString selfC
  marshal (actionsBridge worldSrc machineSrc selfNode)

dmml_fire :: CString -> CString -> CString -> CString -> IO CString
dmml_fire worldC machineC selfC transC = guardedRun $ do
  worldSrc <- T.pack <$> peekCString worldC
  machineSrc <- T.pack <$> peekCString machineC
  selfNode <- T.pack <$> peekCString selfC
  transitionIdent <- T.pack <$> peekCString transC
  marshal (fireBridge worldSrc machineSrc selfNode transitionIdent)

dmml_render_history :: CString -> CString -> IO CString
dmml_render_history historyC machineC = guardedRun $ do
  historyJson <- T.pack <$> peekCString historyC
  machineSrc <- T.pack <$> peekCString machineC
  marshal (renderHistoryBridge historyJson machineSrc)

dmml_actions_history :: CString -> CString -> CString -> IO CString
dmml_actions_history historyC machineC selfC = guardedRun $ do
  historyJson <- T.pack <$> peekCString historyC
  machineSrc <- T.pack <$> peekCString machineC
  selfNode <- T.pack <$> peekCString selfC
  marshal (actionsHistoryBridge historyJson machineSrc selfNode)

dmml_fire_history :: CString -> CString -> CString -> CString -> IO CString
dmml_fire_history historyC machineC selfC transC = guardedRun $ do
  historyJson <- T.pack <$> peekCString historyC
  machineSrc <- T.pack <$> peekCString machineC
  selfNode <- T.pack <$> peekCString selfC
  transitionIdent <- T.pack <$> peekCString transC
  marshal (fireHistoryBridge historyJson machineSrc selfNode transitionIdent)

dmml_render_dir :: CString -> IO CString
dmml_render_dir dirC = guardedRun $ do
  dirPath <- T.pack <$> peekCString dirC
  result <- renderDirBridge dirPath
  marshal result

dmml_actions_dir :: CString -> CString -> IO CString
dmml_actions_dir dirC selfC = guardedRun $ do
  dirPath <- T.pack <$> peekCString dirC
  selfNode <- T.pack <$> peekCString selfC
  result <- actionsDirBridge dirPath selfNode
  marshal result

dmml_fire_dir :: CString -> CString -> CString -> CString -> IO CString
dmml_fire_dir dirC selfC machineC transC = guardedRun $ do
  dirPath <- T.pack <$> peekCString dirC
  selfNode <- T.pack <$> peekCString selfC
  machineNodeText <- T.pack <$> peekCString machineC
  transitionIdent <- T.pack <$> peekCString transC
  result <- fireDirBridge dirPath selfNode machineNodeText transitionIdent
  marshal result

-- | The real exception backstop: runs @act@, and if ANYTHING escapes as
-- a Haskell exception (a parse-library partiality, an out-of-memory, a
-- bug this module doesn't otherwise handle), catches it and returns an
-- @\"ERROR: ...\"@ 'CString' instead of letting it cross into C/JNI as
-- undefined behavior -- see this module's own header comment and
-- dmml/dev-journal/2026-09-04-android-jni-vs-ipc.md.
guardedRun :: IO CString -> IO CString
guardedRun act = do
  result <- try @SomeException act
  case result of
    Right cstr -> pure cstr
    Left e -> newCString ("ERROR: " <> displayException e)
