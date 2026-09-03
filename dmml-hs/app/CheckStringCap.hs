{-# LANGUAGE OverloadedStrings #-}

-- | CLI: does any string-literal fact value in this commit exceed a
-- given character cap? @DMML.StringCap@'s own doc comment explains what
-- real experiment this exists for. Exit 0 (nothing overlong, OR this is
-- a real machine declaration -- machines carry no free-form user
-- content, so there's nothing for this check to find, same "nothing to
-- check" disposition CheckDeclared.hs already gives machine files) or 1
-- (lists every overlong fact found).
--
-- Real bug caught running the dose-response experiment this exists for,
-- fixed here: the first version only ever tried parseCommitSurface, so
-- every legitimate machine file in a mixed commit/machine stream failed
-- with a parse-error-shaped message that the caller (run.py) then
-- miscounted as a genuine string-cap hit -- contaminating the exact
-- metric the experiment measures. Same dual-parse dispatch
-- CheckDeclared.hs already uses, not a new pattern.
--
-- Usage: check-string-cap <max-length> <file.dmml>
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)
import Text.Read (readMaybe)

import DMML.StringCap (OverlongFact (..), overlongStringFacts)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [capStr, path] -> case readMaybe capStr of
      Nothing -> putStrLn ("check-string-cap: not a number: " <> capStr) >> exitFailure
      Just cap -> do
        src <- TIO.readFile path
        case parseCommitSurface src of
          Right commit -> do
            let overlong = overlongStringFacts cap commit
            if null overlong
              then putStrLn ("check-string-cap: OK -- no string literal exceeds " <> show cap <> " characters")
              else do
                putStrLn ("check-string-cap: OVERLONG string literals found (cap " <> show cap <> "):")
                mapM_
                  ( \o ->
                      putStrLn
                        ( "  " <> T.unpack (overlongSubject o) <> " . " <> T.unpack (overlongPredicate o)
                            <> " = <" <> show (overlongLength o) <> " chars>"
                        )
                  )
                  overlong
                exitFailure
          Left commitErr -> case parseMachineSurface src of
            Right _ -> putStrLn "check-string-cap: OK -- machine declaration, nothing to check"
            Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure
    _ -> putStrLn "usage: check-string-cap <max-length> <file.dmml>" >> exitFailure
