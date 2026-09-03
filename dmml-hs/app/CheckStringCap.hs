{-# LANGUAGE OverloadedStrings #-}

-- | CLI: does any string-literal fact value in this commit exceed a
-- given character cap? @DMML.StringCap@'s own doc comment explains what
-- real experiment this exists for. Exit 0 (nothing overlong) or 1
-- (lists every overlong fact found).
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
import DMML.Surface (parseCommitSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [capStr, path] -> case readMaybe capStr of
      Nothing -> putStrLn ("check-string-cap: not a number: " <> capStr) >> exitFailure
      Just cap -> do
        src <- TIO.readFile path
        case parseCommitSurface src of
          Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure
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
    _ -> putStrLn "usage: check-string-cap <max-length> <file.dmml>" >> exitFailure
