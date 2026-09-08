-- | Real, runnable proof that DMML.Jgit + DMML.Checkpoint's
-- resolveAndFoldCheckpoint compose into the actual thing
-- atproto-broker.sh/post-merge used to do in bash: incorporate new
-- commit files (add+commit via JGit), resolve the resulting commits/
-- tree sha, fold a checkpoint against whatever parent existed (or
-- bootstrap if none), write it, and commit the checkpoint file too.
--
-- Usage: jgit-checkpoint-demo <classpath-to-jgit-jar> <throwaway-dir>
module Main (main) where

import DMML.Checkpoint (resolveAndFoldCheckpoint)
import DMML.Jgit
  ( jgitAddFilepattern
  , jgitCommit
  , jgitInit
  , jgitResolve
  , revCommitName
  , JvmEnvironment (..)
  , withJvm
  )
import System.Directory (createDirectoryIfMissing)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [classpath, dir] -> runDemo classpath dir
    _ -> do
      hPutStrLn stderr "usage: jgit-checkpoint-demo <classpath-to-jgit-jar> <throwaway-dir>"
      exitFailure

runDemo :: FilePath -> FilePath -> IO ()
runDemo classpath dir = withJvm (EmbeddedJvm classpath) $ \jvm -> do
  let commitsDir = dir </> "commits"
      checkpointsDir = dir </> "checkpoints"

  putStrLn "=== round 1: genesis, no parent checkpoint (bootstrap) ==="
  git <- jgitInit jvm dir
  createDirectoryIfMissing True commitsDir
  createDirectoryIfMissing True checkpointsDir
  let worldFile = commitsDir </> "world.dmml"
  writeFile worldFile "commit seed\n  declare relation state\n  actor/one `state` idle\n"
  jgitAddFilepattern jvm git "commits/world.dmml"
  _ <- jgitCommit jvm git "seed"

  newTreeSha1 <- jgitResolve jvm git "HEAD:commits"
  case newTreeSha1 of
    Nothing -> putStrLn "FAIL: could not resolve HEAD:commits after genesis commit" >> exitFailure
    Just sha1 -> do
      putStrLn ("commits/ tree sha (round 1) = " <> sha1)
      (ckPath1, folded1) <- resolveAndFoldCheckpoint checkpointsDir commitsDir Nothing sha1 [worldFile]
      putStrLn ("wrote " <> ckPath1 <> ", folded " <> show folded1 <> " file(s) (bootstrap)")
      jgitAddFilepattern jvm git ("checkpoints/" <> sha1 <> ".json")
      commit1 <- jgitCommit jvm git "checkpoint round 1"
      name1 <- revCommitName jvm commit1
      putStrLn ("committed checkpoint, SHA = " <> name1)

      putStrLn "\n=== round 2: real parent checkpoint exists, fold only the new file ==="
      let newFile = commitsDir </> "fire-1.dmml"
      writeFile newFile "commit fire1\n  actor/one `state` awake\n"
      jgitAddFilepattern jvm git "commits/fire-1.dmml"
      _ <- jgitCommit jvm git "fire1"

      newTreeSha2 <- jgitResolve jvm git "HEAD:commits"
      case newTreeSha2 of
        Nothing -> putStrLn "FAIL: could not resolve HEAD:commits after round 2 commit" >> exitFailure
        Just sha2 -> do
          putStrLn ("commits/ tree sha (round 2) = " <> sha2)
          (ckPath2, folded2) <- resolveAndFoldCheckpoint checkpointsDir commitsDir (Just sha1) sha2 [newFile]
          putStrLn ("wrote " <> ckPath2 <> ", folded " <> show folded2 <> " file(s) (incremental, parent = " <> sha1 <> ")")
          if folded2 == 1
            then putStrLn "SUCCESS: round 2 folded exactly the 1 new file, not a full replay"
            else putStrLn ("FAIL: expected exactly 1 file folded in round 2, got " <> show folded2) >> exitFailure
