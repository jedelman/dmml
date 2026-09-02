-- | Pure Shannon-entropy bookkeeping for detecting entropic collapse in
-- a (subject, predicate) pair's live alternatives -- deliberately
-- dependency-light (no aeson, no IO), so it stays a small, reusable
-- piece independent of whatever a caller uses for persistence.
--
-- Design (from the 2026-09-02 conversation this implements): entropy is
-- computed from real, undeduped assertion counts -- every individual
-- commit that ever asserted a value for a pair, not the collapsed
-- 'DMML.Materialize.Alternatives' view (which dedupes by value and
-- would erase exactly the vote-weighting entropy needs). "Collapse" is
-- never about absolute entropy or its long-run trend -- climbing
-- entropy, forever, on an ungoverned pair is fine (Jason,
-- 2026-09-02: "let them fight"). What matters is a RAPID, LOCAL drop
-- within a sliding window -- and even that is only meaningful once
-- checked against whether a real governed resolution
-- ('DMML.Governance.arbitrate' returning 'DMML.Governance.Resolved')
-- actually explains it. This module only computes the entropy signal;
-- attribution is the caller's job (it needs 'DMML.Governance', which
-- would make this module depend on the whole materializer for no
-- reason if it lived here too).
module DMML.Entropy
  ( shannonEntropy
  , RoundSample (..)
  , WindowedHistory
  , recordSample
  , windowDelta
  ) where

import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)

-- | Shannon entropy, in bits, of a value-count distribution. 0 for an
-- empty or single-valued distribution (full consensus, no disagreement
-- to have an entropy about).
shannonEntropy :: Map Text Int -> Double
shannonEntropy counts
  | total <= 0 = 0
  | otherwise =
      negate (sum [p * logBase 2 p | c <- Map.elems counts, c > 0, let p = fromIntegral c / fromIntegral total])
  where
    total = sum (Map.elems counts)

-- | One entropy reading, tagged with a position in the commit sequence
-- (a file index, not a wall-clock time -- the substrate's own ordering
-- is the only "time" this needs).
data RoundSample = RoundSample {sampleIndex :: Int, sampleEntropy :: Double}
  deriving (Eq, Show)

-- | Most-recent-first, capped at the configured window size by
-- 'recordSample' -- bounded memory regardless of how long a corpus runs.
type WindowedHistory = [RoundSample]

recordSample :: Int -> Int -> Double -> WindowedHistory -> WindowedHistory
recordSample windowSize idx h history = take windowSize (RoundSample idx h : history)

-- | Entropy change between the most recent sample and the OLDEST sample
-- still inside the window -- 'Nothing' until the window actually holds
-- more than one sample (a fresh pair has nothing to compare against
-- yet, which is correct: a brand-new disagreement is not a collapse).
-- Negative means entropy fell (alternatives became less diverse);
-- how negative is "collapse" is a threshold the caller applies, not
-- this function's business.
windowDelta :: WindowedHistory -> Maybe Double
windowDelta history = case history of
  (newest : rest@(_ : _)) -> Just (sampleEntropy newest - sampleEntropy (last rest))
  _ -> Nothing
