{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ScopedTypeVariables #-}

-- | The same interactive loop as 'app/InteractiveBrowser.hs' -- built on
-- exactly the same primitives ('DMML.Materialize'\/'DMML.Guard'\/
-- 'DMML.Fire', accumulating each fired transition as its own real,
-- content-addressed 'IdentifiedCommit') -- driven by tapped links in a
-- browser instead of typed numbers at a terminal prompt. No JavaScript,
-- no client-side state: every tap is a plain GET to @\/fire\/\<ident\>@,
-- the server re-materializes and redirects back to @\/@, the same
-- request-response shape any plain HTML browser (including a touch
-- one, with no keyboard involved at all) already handles natively.
--
-- Deliberately a hand-rolled, single-connection-at-a-time HTTP\/1.1
-- server over 'Network.Socket' rather than a real web framework (warp\/
-- wai) -- this is a single-player, single-session, localhost-only
-- prototype; the smallest real thing that proves the "touch, don't
-- type" shape, not a real web server. State lives in one 'IORef',
-- exactly like the terminal REPL's own accumulated history, just read\/
-- written by request handlers instead of a stdin loop.
--
-- Usage: touch-browser <world.dmml> <machine.dmml> [port]
module Main (main) where

import Control.Exception (SomeException, bracket, catch)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC
import Data.IORef
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import Network.Socket
import Network.Socket.ByteString (recv, sendAll)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt, NodeRef (..), machineNode, nodeRefSegments)
import DMML.Fire (fireTransition, renderFiredCommit)
import DMML.Guard (EvalContext (..), availableTransitions)
import DMML.LocalIdentity (localFileRef)
import DMML.Materialize (IdentifiedCommit (..), applyIdentifiedCommits, renderSnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

selfNode :: Text
selfNode = "player/one"

main :: IO ()
main = do
  args <- getArgs
  (worldPath, machinePath, port) <- case args of
    [w, m] -> pure (w, m, "8765")
    [w, m, p] -> pure (w, m, p)
    _ -> putStrLn "usage: touch-browser <world.dmml> <machine.dmml> [port]" >> exitFailure >> error "unreachable"

  worldRaw <- BS.readFile worldPath
  worldStmt <- case parseCommitSurface (TE.decodeUtf8 worldRaw) of
    Right stmt -> pure stmt
    Left err -> putStrLn (worldPath <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"
  machine <- do
    src <- TIO.readFile machinePath
    case parseMachineSurface src of
      Right m -> pure m
      Left err -> putStrLn (machinePath <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"

  let initial = IdentifiedCommit {icRef = localFileRef worldPath worldRaw, icCommit = worldStmt}
  historyRef <- newIORef ([initial], 1 :: Int)

  let hints = defaultHints {addrFlags = [AI_PASSIVE], addrSocketType = Stream}
  addrs <- getAddrInfo (Just hints) (Just "127.0.0.1") (Just port)
  let addr = head addrs
  bracket (openServerSocket addr) close $ \sock -> do
    putStrLn ("touch-browser: listening on http://127.0.0.1:" <> port <> "/ (Ctrl+C to stop)")
    serverLoop sock machine historyRef

openServerSocket :: AddrInfo -> IO Socket
openServerSocket addr = do
  sock <- socket (addrFamily addr) (addrSocketType addr) (addrProtocol addr)
  setSocketOption sock ReuseAddr 1
  bind sock (addrAddress addr)
  listen sock 8
  pure sock

serverLoop :: Socket -> MachineStmt -> IORef ([IdentifiedCommit], Int) -> IO ()
serverLoop sock machine historyRef = loop
  where
    loop = do
      (conn, _peer) <- accept sock
      handleConn conn machine historyRef `catch` (\(_ :: SomeException) -> pure ())
      close conn
      loop

handleConn :: Socket -> MachineStmt -> IORef ([IdentifiedCommit], Int) -> IO ()
handleConn conn machine historyRef = do
  req <- recv conn 8192
  let requestLine = BC.takeWhile (/= '\r') req
      parts = BC.words requestLine
  case parts of
    (_method : path : _rest) -> route conn machine historyRef (BC.unpack path)
    _ -> respond conn "400 Bad Request" "text/plain" "bad request"

route :: Socket -> MachineStmt -> IORef ([IdentifiedCommit], Int) -> String -> IO ()
route conn machine historyRef path
  | path == "/" || path == "" = renderPage conn machine historyRef Nothing
  | Just ident <- stripPrefixStr "/fire/" path = do
      result <- fireOne machine historyRef (T.pack ident)
      case result of
        Right () -> redirect conn "/"
        Left err -> renderPage conn machine historyRef (Just err)
  | otherwise = respond conn "404 Not Found" "text/plain" "not found"

stripPrefixStr :: String -> String -> Maybe String
stripPrefixStr prefix s
  | take (length prefix) s == prefix = Just (drop (length prefix) s)
  | otherwise = Nothing

fireOne :: MachineStmt -> IORef ([IdentifiedCommit], Int) -> Text -> IO (Either String ())
fireOne machine historyRef transitionIdent = do
  (history, round_) <- readIORef historyRef
  let snap = applyIdentifiedCommits "world" history
      machineKey = nodeRefText (machineNode machine)
      machineMap = Map.singleton machineKey machine
      ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
  case fireTransition machineMap machine transitionIdent ctx snap of
    Left err -> pure (Left (show err))
    Right effects -> do
      let commitLabel = "fire" <> T.pack (show round_)
          commitText = renderFiredCommit commitLabel effects
      case parseCommitSurface commitText of
        Left _ -> do
          -- Legal but no real effect (e.g. a guard-only transition) --
          -- nothing to append, just advance the round counter so the
          -- next real fire gets a fresh label.
          writeIORef historyRef (history, round_ + 1)
          pure (Right ())
        Right stmt -> do
          let newRef = localFileRef (T.unpack commitLabel) (TE.encodeUtf8 commitText)
              newCommit = IdentifiedCommit {icRef = newRef, icCommit = stmt}
          writeIORef historyRef (history ++ [newCommit], round_ + 1)
          pure (Right ())

renderPage :: Socket -> MachineStmt -> IORef ([IdentifiedCommit], Int) -> Maybe String -> IO ()
renderPage conn machine historyRef mErr = do
  (history, _round) <- readIORef historyRef
  let snap = applyIdentifiedCommits "world" history
      machineKey = nodeRefText (machineNode machine)
      machineMap = Map.singleton machineKey machine
      ctx = EvalContext {ctxSelfNode = selfNode, ctxParams = Map.empty}
      actions = availableTransitions machineMap ctx snap
      rendered = renderSnapshot snap
      buttons =
        if null actions
          then "<p class=\"muted\">Nothing more you can do here.</p>"
          else T.concat [button t | (_m, t) <- actions]
      errBlock = maybe "" (\e -> "<p class=\"err\">refused: " <> T.pack e <> "</p>") mErr
      page = pageTemplate rendered buttons errBlock
  respond conn "200 OK" "text/html; charset=utf-8" (TE.encodeUtf8 page)
  where
    button t = "<a class=\"btn\" href=\"/fire/" <> t <> "\">" <> t <> "</a>\n"

pageTemplate :: Text -> Text -> Text -> Text
pageTemplate rendered buttons errBlock =
  T.unlines
    [ "<!doctype html><html><head><meta charset=\"utf-8\">"
    , "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
    , "<title>dmml touch-browser</title>"
    , "<style>"
    , "  body{font-family:sans-serif;margin:0;padding:1rem;background:#111;color:#eee;}"
    , "  pre{white-space:pre-wrap;background:#1b1b1b;padding:0.75rem;border-radius:0.5rem;}"
    , "  .btn{display:block;margin:0.6rem 0;padding:1.1rem 1rem;font-size:1.2rem;"
    , "       text-align:center;background:#2d6cdf;color:#fff;text-decoration:none;"
    , "       border-radius:0.6rem;}"
    , "  .btn:active{background:#1f4fa8;}"
    , "  .muted{color:#888;}"
    , "  .err{color:#e88;}"
    , "</style></head><body>"
    , "<h2>dmml -- touch browser</h2>"
    , errBlock
    , "<pre>" <> rendered <> "</pre>"
    , buttons
    , "</body></html>"
    ]

respond :: Socket -> BS.ByteString -> BS.ByteString -> BS.ByteString -> IO ()
respond conn status contentType body = do
  let headers =
        BC.concat
          [ "HTTP/1.1 ", status, "\r\n"
          , "Content-Type: ", contentType, "\r\n"
          , "Content-Length: ", BC.pack (show (BS.length body)), "\r\n"
          , "Connection: close\r\n\r\n"
          ]
  sendAll conn (headers <> body)

redirect :: Socket -> String -> IO ()
redirect conn location = do
  let headers =
        BC.concat
          [ "HTTP/1.1 302 Found\r\n"
          , "Location: ", BC.pack location, "\r\n"
          , "Content-Length: 0\r\n"
          , "Connection: close\r\n\r\n"
          ]
  sendAll conn headers
