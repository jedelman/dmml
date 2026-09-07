-- | Real, runnable proof that DMML.Jgit's typed wrappers actually work
-- end to end -- not just that they compile. Embeds a JVM, inits a
-- throwaway repo, writes a file, adds it, commits it, reads the
-- resulting commit's name back, and independently verifies the result
-- with the real @git@ CLI (a second, unrelated implementation checking
-- the first one's work, not JGit grading its own homework).
--
-- Usage: jgit-smoke-test <classpath-to-jgit-jar> <throwaway-dir>
module Main (main) where

import DMML.Jgit
  ( jgitAddFilepattern
  , jgitCommit
  , jgitInit
  , revCommitName
  , withEmbeddedJvm
  )
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [classpath, dir] -> runSmokeTest classpath dir
    _ -> do
      hPutStrLn stderr "usage: jgit-smoke-test <classpath-to-jgit-jar> <throwaway-dir>"
      exitFailure

runSmokeTest :: FilePath -> FilePath -> IO ()
runSmokeTest classpath dir = withEmbeddedJvm classpath $ \jvm -> do
  putStrLn ("jgit-smoke-test: Git.init() at " <> dir)
  git <- jgitInit jvm dir

  let filePath = dir <> "/hello.txt"
  writeFile filePath "hello from DMML.Jgit\n"
  putStrLn ("jgit-smoke-test: wrote " <> filePath)

  putStrLn "jgit-smoke-test: Git.add().addFilepattern(\"hello.txt\").call()"
  jgitAddFilepattern jvm git "hello.txt"

  putStrLn "jgit-smoke-test: Git.commit().setMessage(...).call()"
  revCommit <- jgitCommit jvm git "real commit via DMML.Jgit"

  name <- revCommitName jvm revCommit
  putStrLn ("jgit-smoke-test: commit name (SHA) = " <> name)
  putStrLn "jgit-smoke-test: SUCCESS"
