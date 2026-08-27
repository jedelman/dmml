//! A real `AppendSubstrate` implementation backed by a live `iroh-docs` `Doc`.
//!
//! One `Doc` == one DMML world/namespace. Commits are content-addressed by
//! BLAKE3-of-JSON-CID (see `commit_cid`), and that CID string is used directly
//! as the iroh-docs entry key, so distinct commits from the same author can
//! never overwrite each other.

use std::future::Future;

use iroh_blobs::api::Store;
use iroh_docs::api::Doc;
use iroh_docs::store::Query;
use iroh_docs::{AuthorId, Entry};
use n0_future::StreamExt;
use oxigraph::model::NamedOrBlankNode;

use dmml_runtime::graph::{parse_nquads, Commit, ConsumeRef, FactRef};
use dmml_runtime::substrate::{
    AppendSubstrate, Assertion, CommitReceipt, CommitRecord, RetractionStatus, Substrate,
};

/// Error type mirroring `MockError`'s shape: a plain string wrapper.
///
/// iroh's APIs return `anyhow::Result`, so we convert at each `?` boundary
/// via `From<anyhow::Error>` (using its `Display` output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohSubstrateError(pub String);

impl std::fmt::Display for IrohSubstrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for IrohSubstrateError {}

impl From<anyhow::Error> for IrohSubstrateError {
    fn from(err: anyhow::Error) -> Self {
        IrohSubstrateError(err.to_string())
    }
}

/// An `AppendSubstrate` bound to one live iroh-docs `Doc` and one blob store.
///
/// Note: the `CommitReceipt.cid` returned by `append_commit` is the
/// BLAKE3-of-JSON CID we compute ourselves — NOT iroh's blob `Hash` of the
/// raw entry bytes (a different, also-real hash; don't conflate the two).
/// Our key scheme means the CID is recoverable directly from `entry.key()`.
pub struct IrohAppendSubstrate {
    owner: AuthorId,
    namespace: String,
    doc: Doc,
    blobs: Store,
}

impl IrohAppendSubstrate {
    /// Bind to an existing `Doc` (== one DMML world) with a default author
    /// identity and a clone of the same `Store` handle that was passed to
    /// `Docs::memory().spawn(...)` (i.e. the `(*blobs).clone()` value).
    pub fn new(owner: AuthorId, namespace: String, doc: Doc, blobs: Store) -> Self {
        Self {
            owner,
            namespace,
            doc,
            blobs,
        }
    }

    /// Fetch and deserialize the `Commit` stored at an entry.
    async fn entry_commit(&self, entry: &Entry) -> Result<Commit, IrohSubstrateError> {
        let bytes = self
            .blobs
            .blobs()
            .get_bytes(entry.content_hash())
            .await
            .map_err(|e| IrohSubstrateError(format!("blob read failed: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| IrohSubstrateError(format!("commit deserialization failed: {e}")))
    }

    /// Enumerate every entry in the doc as `(cid, Commit)` pairs.
    async fn all_commits(&self) -> Result<Vec<(String, Commit)>, IrohSubstrateError> {
        // `Query::all()` returns a `QueryBuilder<FlatQuery>` which converts
        // into `Query` via `Into<Query>`; the direct form is the common usage.
        let stream = self.doc.get_many(Query::all()).await?;
        tokio::pin!(stream);
        let mut out = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            let cid = String::from_utf8_lossy(entry.key()).into_owned();
            let commit = self.entry_commit(&entry).await?;
            out.push((cid, commit));
        }
        Ok(out)
    }

    /// Mirror of `mock.rs`'s exact `cites()` matching semantics.
    fn cites(consumed: &FactRef, fact: &FactRef) -> bool {
        consumed.commit == fact.commit
            && consumed.subject == fact.subject
            && consumed.predicate == fact.predicate
            && match (&consumed.object, &fact.object) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            }
    }
}

impl Substrate for IrohAppendSubstrate {
    type Identity = AuthorId;
    type Error = IrohSubstrateError;

    fn name(&self) -> &'static str {
        "iroh-append"
    }

    fn owner(&self) -> &Self::Identity {
        &self.owner
    }

    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn get_commit(
        &self,
        cid: &str,
    ) -> impl Future<Output = Result<Option<CommitRecord>, Self::Error>> + Send {
        async move {
            // `get_exact` needs a specific author, but a CID alone doesn't
            // tell us which author wrote it. Since the key IS the
            // content-derived CID, `Query::key_exact` finds it regardless of
            // author (two authors producing byte-identical commits is a
            // harmless collision — they're semantically identical).
            let stream = self.doc.get_many(Query::key_exact(cid.as_bytes())).await?;
            tokio::pin!(stream);
            match stream.next().await {
                None => Ok(None),
                Some(entry) => {
                    let entry = entry?;
                    let commit = self.entry_commit(&entry).await?;
                    Ok(Some(CommitRecord {
                        cid: cid.to_string(),
                        commit,
                    }))
                }
            }
        }
    }

    fn resolve_fact(
        &self,
        fact: &FactRef,
    ) -> impl Future<Output = Result<RetractionStatus, Self::Error>> + Send {
        async move {
            let mut by = Vec::new();
            for (cid, commit) in self.all_commits().await? {
                for consumed in &commit.consumes {
                    if let ConsumeRef::Fact(consumed_fact) = consumed {
                        if Self::cites(consumed_fact, fact) {
                            by.push(cid.clone());
                        }
                    }
                }
            }
            if by.is_empty() {
                Ok(RetractionStatus::Live)
            } else {
                Ok(RetractionStatus::Retracted { by })
            }
        }
    }

    fn assertions(
        &self,
        subject: &oxigraph::model::NamedNode,
        predicate: &oxigraph::model::NamedNode,
    ) -> impl Future<Output = Result<Vec<Assertion>, Self::Error>> + Send {
        async move {
            let mut out = Vec::new();
            for (cid, commit) in self.all_commits().await? {
                let quads = parse_nquads(&commit.produces).map_err(|e| {
                    IrohSubstrateError(format!("n-quads parse failed for commit {cid}: {e}"))
                })?;
                // Mirrors mock.rs's exact matching: `subject` is
                // NamedOrBlankNode (only the NamedNode variant can match a
                // NamedNode query subject), `predicate` is a bare NamedNode
                // (never wrapped in Term) -- NOT `Term::NamedNode(..)` for
                // either, despite superficially resembling that shape.
                for q in quads {
                    let subject_matches = match &q.subject {
                        NamedOrBlankNode::NamedNode(n) => n == subject,
                        NamedOrBlankNode::BlankNode(_) => false,
                    };
                    if subject_matches && &q.predicate == predicate {
                        out.push(Assertion {
                            commit: cid.clone(),
                            object: q.object.clone(),
                        });
                    }
                }
            }
            Ok(out)
        }
    }
}

impl AppendSubstrate for IrohAppendSubstrate {
    fn append_commit(
        &self,
        author: &Self::Identity,
        commit: &Commit,
    ) -> impl Future<Output = Result<CommitReceipt, Self::Error>> + Send {
        async move {
            let json_bytes = serde_json::to_vec(commit)
                .map_err(|e| IrohSubstrateError(format!("commit serialization failed: {e}")))?;
            let cid = blake3::hash(&json_bytes).to_hex().to_string();
            // `set_bytes`'s `key: impl Into<Bytes>` needs owned/'static data,
            // not a borrow of `cid` -- `cid.clone().into_bytes()` (String ->
            // Vec<u8>) satisfies that via `bytes::Bytes`'s own `From<Vec<u8>>`.
            // `AuthorId` is `Copy`; pass it by value as `set_bytes` requires.
            self.doc
                .set_bytes(*author, cid.clone().into_bytes(), json_bytes)
                .await?;
            // NOTE: the returned `Hash` from `set_bytes` is iroh's blob hash
            // of the raw bytes — deliberately NOT used as our `CommitReceipt.cid`.
            Ok(CommitReceipt { cid })
        }
    }
}
