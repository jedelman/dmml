{-# LANGUAGE OverloadedStrings #-}

-- | A real, deterministic, local content fingerprint for a @.dmml@
-- file -- NOT the real production substrate's identity scheme. The real
-- Rust crate computes an actual @CIDv1(dag-cbor, sha2-256)@ over a
-- commit's canonical DAG-CBOR encoding (@dmml::identity@,
-- @written-world/SPEC.md@'s "real, independently-operated PDS computed
-- the byte-identical CID" section) -- this toolchain has no SHA-2
-- library available (`ghc-pkg list` here carries no `cryptohash`/
-- `crypton`/`SHA` package, and per this repo's own README, no Hackage
-- access to fetch one), and no CBOR canonicalization either. Rather than
-- fake a `sha256:`-labeled string that ISN'T actually SHA-256 (which
-- would be worse than admitting the gap -- a false compatibility claim,
-- not just a missing feature), this hashes the exact bytes read from
-- disk with a real, simple, well-defined algorithm (FNV-1a, 64-bit) and
-- labels the result honestly as what it is.
--
-- What this buys: a real 'DMML.Ast.StrongRef' whose @cid@ actually
-- changes when the file's content changes and stays stable when it
-- doesn't -- exactly what 'DMML.Fire'\'s retract-provenance path needs
-- (jedelman/dmml#4) to build a real, re-checkable @consumes@ citation
-- instead of refusing to fire a retract effect at all. What this does
-- NOT buy: interoperability with a real atproto-issued CID for the same
-- content, or cryptographic collision resistance -- this identifies a
-- LOCAL FILE'S bytes to THIS toolchain, nothing more, and the @local:@/
-- @fnv1a64:@ prefixes below say so wherever the identity is used.
module DMML.LocalIdentity
  ( localFileRef
  , fnv1a64Hex
  ) where

import Data.Bits (xor)
import qualified Data.ByteString as BS
import Data.Text (Text)
import qualified Data.Text as T
import Data.Word (Word64)
import Numeric (showHex)

import DMML.Ast (Span (..), StrongRef (..))

fnv1a64OffsetBasis :: Word64
fnv1a64OffsetBasis = 14695981039346656037

fnv1a64Prime :: Word64
fnv1a64Prime = 1099511628211

-- | FNV-1a, 64-bit, over raw bytes -- a real, standard, publicly
-- specified algorithm (not invented for this module), chosen because it
-- needs nothing beyond 'Data.Bits'\/'Data.Word', both already available
-- with no extra package. Rendered as 16 lowercase hex digits.
fnv1a64Hex :: BS.ByteString -> Text
fnv1a64Hex bs = T.pack (pad16 (showHex (BS.foldl' step fnv1a64OffsetBasis bs) ""))
  where
    step acc byte = (acc `xor` fromIntegral byte) * fnv1a64Prime
    pad16 s = replicate (16 - length s) '0' <> s

-- | A real 'StrongRef' identifying one file's exact current bytes: the
-- file's own path as @uri@ (prefixed @local:@ -- this is a filesystem
-- path, not a real network-resolvable URI), and its FNV-1a-64 fingerprint
-- as @cid@ (prefixed @fnv1a64:@, never @sha256:@ -- see this module's
-- own doc comment for why that distinction matters). Two calls on files
-- with byte-identical content always produce the same @cid@, whatever
-- their paths -- this is real content addressing, just not the
-- production substrate's own scheme.
localFileRef :: FilePath -> BS.ByteString -> StrongRef
localFileRef path contents =
  StrongRef
    { strongRefUri = "local:" <> T.pack path
    , strongRefCid = "fnv1a64:" <> fnv1a64Hex contents
    , strongRefSpan = Span "<local-identity>"
    }
