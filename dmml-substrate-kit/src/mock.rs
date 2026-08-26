//! An in-memory, zero-network `Substrate` implementation -- the "next
//! step, not yet built" this crate's own `lib.rs` doc comment named.
//! Its whole purpose is testing `dmml-runtime`'s trait usage (and any
//! calling code built against `Substrate`) without a real PDS or a real
//! iroh network, the same role `client/examples/mock_pds.rs` already
//! plays for atproto in written-world.
//!
//! Implements [`AppendSubstrate`] only, not [`CasSubstrate`] --
//! deliberately: this mock stands in for the iroh shape (author-
//! partitioned, no admission gate), the harder of the two to get right
//! since detection is the caller's job. A CAS-shaped mock (standing in
//! for atproto) is a real, separate next step if one's ever needed; see
//! `dmml_runtime::substrate`'s own doc comment for why a single type
//! implementing both capability traits would be the wrong shape anyway
//! (atproto's write is inherently gated -- an `AppendSubstrate` impl for
//! it would be dishonest).
//!
//! CIDs here are a synthesized, incrementing token
//! (`"mock-cid-{n}"`) -- not a real hash of anything. `dmml_runtime`'s
//! own contract treats a CID as an opaque, comparable string it never
//! re-derives or verifies, so a synthetic-but-unique token satisfies
//! that contract exactly as well as a real hash would for testing
//! purposes; producing a real one is `atproto_cid`/a future `iroh_cid`
//! module's job, not this mock's.

use std::sync::RwLock;

use oxigraph::model::{NamedNode, NamedOrBlankNode};

use dmml_runtime::graph::{parse_nquads, Commit, ConsumeRef, FactRef};
use dmml_runtime::substrate::{
    AppendSubstrate, Assertion, CommitReceipt, CommitRecord, RetractionStatus, Substrate,
};

/// An opaque author/owner identity for the mock -- just a label. A real
/// backend's `Identity` carries actual credential material; this one
/// carries nothing because there's no real network call to authenticate
/// to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockIdentity(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockError(pub String);

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MockError {}

struct StoredCommit {
    cid: String,
    author: MockIdentity,
    commit: Commit,
}

/// A single world's worth of author-partitioned, in-memory commits.
/// `owner`/`namespace` are the sovereignty root this instance is bound
/// to for the lifetime of the value -- there is no construction path
/// that lets a caller point one instance at more than one world.
pub struct MockAppendSubstrate {
    owner: MockIdentity,
    namespace: String,
    log: RwLock<Vec<StoredCommit>>,
    next_cid: RwLock<u64>,
}

impl MockAppendSubstrate {
    pub fn new(owner: MockIdentity, namespace: impl Into<String>) -> Self {
        Self {
            owner,
            namespace: namespace.into(),
            log: RwLock::new(Vec::new()),
            next_cid: RwLock::new(0),
        }
    }

    fn fresh_cid(&self) -> String {
        let mut n = self.next_cid.write().expect("mock lock poisoned");
        let cid = format!("mock-cid-{n}");
        *n += 1;
        cid
    }

    /// Every commit CID appended under `author`, in append order --
    /// exercises the `AppendSubstrate` contract's own partitioning claim
    /// ("never touches another author's entries") rather than leaving it
    /// asserted only in the trait's doc comment.
    pub fn commits_by(&self, author: &MockIdentity) -> Vec<String> {
        self.log
            .read()
            .expect("mock lock poisoned")
            .iter()
            .filter(|c| &c.author == author)
            .map(|c| c.cid.clone())
            .collect()
    }

    /// Whether a stored commit's own `consumes` cites `fact` -- the
    /// exact `(commit, subject, predicate)` triple, honoring `FactRef`'s
    /// own wildcard-object semantics (an omitted `object` on either side
    /// matches any object, per `dmml-runtime`'s `FactRef` doc comment).
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

impl Substrate for MockAppendSubstrate {
    type Identity = MockIdentity;
    type Error = MockError;

    fn name(&self) -> &'static str {
        "mock-append"
    }

    fn owner(&self) -> &Self::Identity {
        &self.owner
    }

    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn get_commit(&self, cid: &str) -> Result<Option<CommitRecord>, Self::Error> {
        let log = self.log.read().expect("mock lock poisoned");
        Ok(log
            .iter()
            .find(|c| c.cid == cid)
            .map(|c| CommitRecord {
                cid: c.cid.clone(),
                commit: c.commit.clone(),
            }))
    }

    async fn resolve_fact(&self, fact: &FactRef) -> Result<RetractionStatus, Self::Error> {
        let log = self.log.read().expect("mock lock poisoned");
        let by: Vec<String> = log
            .iter()
            .filter(|c| {
                c.commit.consumes.iter().any(|r| match r {
                    ConsumeRef::Fact(f) => Self::cites(f, fact),
                    ConsumeRef::Strong(_) => false,
                })
            })
            .map(|c| c.cid.clone())
            .collect();
        Ok(if by.is_empty() {
            RetractionStatus::Live
        } else {
            RetractionStatus::Retracted { by }
        })
    }

    async fn assertions(
        &self,
        subject: &NamedNode,
        predicate: &NamedNode,
    ) -> Result<Vec<Assertion>, Self::Error> {
        let log = self.log.read().expect("mock lock poisoned");
        let mut out = Vec::new();
        for c in log.iter() {
            let quads = parse_nquads(&c.commit.produces)
                .map_err(|e| MockError(format!("stored commit {} has malformed produces: {e}", c.cid)))?;
            for q in quads {
                let subject_matches = match &q.subject {
                    NamedOrBlankNode::NamedNode(n) => n == subject,
                    NamedOrBlankNode::BlankNode(_) => false,
                };
                if subject_matches && &q.predicate == predicate {
                    out.push(Assertion {
                        commit: c.cid.clone(),
                        object: q.object.clone(),
                    });
                }
            }
        }
        Ok(out)
    }
}

impl AppendSubstrate for MockAppendSubstrate {
    async fn append_commit(
        &self,
        author: &Self::Identity,
        commit: &Commit,
    ) -> Result<CommitReceipt, Self::Error> {
        let cid = self.fresh_cid();
        let mut log = self.log.write().expect("mock lock poisoned");
        log.push(StoredCommit {
            cid: cid.clone(),
            author: author.clone(),
            commit: commit.clone(),
        });
        Ok(CommitReceipt { cid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmml_runtime::graph::StrongRef;

    fn commit(consumes: Vec<ConsumeRef>, produces: &str) -> Commit {
        Commit {
            consumes,
            produces: produces.to_string(),
            predicate: "asserts".to_string(),
            via: None,
            responds_to: None,
            created_at: "2026-08-26T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn a_bare_produces_never_retracts_anything() {
        let sub = MockAppendSubstrate::new(MockIdentity("did:example:alice".into()), "world/1");
        let author = MockIdentity("author:device-a".into());

        sub.append_commit(
            &author,
            &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."),
        )
        .await
        .unwrap();

        let fact = FactRef {
            commit: StrongRef {
                uri: "at://did:example:alice/x/rkey1".into(),
                cid: "mock-cid-0".into(),
            },
            subject: "sky/1".into(),
            predicate: "origin".into(),
            object: None,
        };
        assert_eq!(sub.resolve_fact(&fact).await.unwrap(), RetractionStatus::Live);
    }

    #[tokio::test]
    async fn two_commits_consuming_the_same_base_is_the_real_conflict_signature() {
        let sub = MockAppendSubstrate::new(MockIdentity("did:example:alice".into()), "world/1");
        let device_a = MockIdentity("author:device-a".into());
        let device_b = MockIdentity("author:device-b".into());

        // A base fact, produced by an earlier commit (real CID assigned
        // by the mock itself, not hand-picked, so this genuinely proves
        // resolve_fact against the substrate's own generated CIDs).
        let base_receipt = sub
            .append_commit(&device_a, &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."))
            .await
            .unwrap();

        let base = FactRef {
            commit: StrongRef {
                uri: "at://did:example:alice/x/rkey-base".into(),
                cid: base_receipt.cid.clone(),
            },
            subject: "sky/1".into(),
            predicate: "origin".into(),
            object: None,
        };

        // Device A and device B each independently consume the SAME
        // base, unaware of each other -- the one real conflict shape
        // per dmml/ARCHITECTURE.md.
        sub.append_commit(
            &device_a,
            &commit(
                vec![ConsumeRef::Fact(base.clone())],
                "<x:sky/1> <x:origin> \"duskweave\" .",
            ),
        )
        .await
        .unwrap();
        sub.append_commit(
            &device_b,
            &commit(
                vec![ConsumeRef::Fact(base.clone())],
                "<x:sky/1> <x:origin> \"moonwoven\" .",
            ),
        )
        .await
        .unwrap();

        match sub.resolve_fact(&base).await.unwrap() {
            RetractionStatus::Retracted { by } => {
                assert_eq!(by.len(), 2, "both concurrent consumers should show up: {by:?}");
            }
            RetractionStatus::Live => panic!("a twice-consumed base must not read as Live"),
        }
    }

    #[tokio::test]
    async fn writes_are_partitioned_per_author_by_construction() {
        let sub = MockAppendSubstrate::new(MockIdentity("did:example:alice".into()), "world/1");
        let device_a = MockIdentity("author:device-a".into());
        let device_b = MockIdentity("author:device-b".into());

        let a1 = sub
            .append_commit(&device_a, &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."))
            .await
            .unwrap();
        let b1 = sub
            .append_commit(&device_b, &commit(vec![], "<x:sky/1> <x:origin> \"moonwoven\" ."))
            .await
            .unwrap();
        let a2 = sub
            .append_commit(&device_a, &commit(vec![], "<x:sky/2> <x:origin> \"rosefingered\" ."))
            .await
            .unwrap();

        assert_eq!(sub.commits_by(&device_a), vec![a1.cid, a2.cid]);
        assert_eq!(sub.commits_by(&device_b), vec![b1.cid]);
    }

    #[tokio::test]
    async fn assertions_returns_every_independent_production_not_just_the_last() {
        let sub = MockAppendSubstrate::new(MockIdentity("did:example:alice".into()), "world/1");
        let a = MockIdentity("author:helios".into());
        let b = MockIdentity("author:selene".into());

        sub.append_commit(&a, &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."))
            .await
            .unwrap();
        sub.append_commit(&b, &commit(vec![], "<x:sky/1> <x:origin> \"moonwoven\" ."))
            .await
            .unwrap();

        let subject = NamedNode::new("x:sky/1").unwrap();
        let predicate = NamedNode::new("x:origin").unwrap();
        let found = sub.assertions(&subject, &predicate).await.unwrap();
        assert_eq!(found.len(), 2, "both independent assertions must coexist: {found:?}");
    }
}
