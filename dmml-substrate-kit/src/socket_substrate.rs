//! A real, out-of-process `AppendSubstrate` over plain TCP sockets --
//! not a hash-quality or performance stand-in for `iroh_substrate`, a
//! **build-weight** one. `iroh`/`iroh-docs`/`iroh-blobs`/`iroh-gossip`
//! pull in QUIC, relay discovery, netlink, and their own crypto stacks;
//! linking `dmml-substrate-kit`'s examples against that graph is what
//! exhausted this sandbox's disk mid-build (real, observed: `rustc-LLVM
//! ERROR: IO failure on output stream: No space left on device` and a
//! `Bus error` from the linker on `pantheon_commons*`). `mock.rs`
//! already sidesteps all of that, but it's in-process only (one
//! `RwLock<Vec<StoredCommit>>` shared by direct method calls) -- it
//! can't exercise "two independent authors, two independent
//! connections" the way a real multi-writer substrate has to. This
//! module is that: the same author-partitioned, no-admission-gate
//! `AppendSubstrate` shape iroh-docs implements, over a server any
//! number of real `TcpStream` clients can dial into, with zero
//! dependency beyond `tokio` (already required) and `serde_json`
//! (already required for the JSON commit encoding `iroh_substrate.rs`
//! also uses).
//!
//! Gated behind the `sockets` feature so a local sanity build can skip
//! `iroh` entirely: `cargo test -p dmml-substrate-kit --no-default-
//! features --features sockets`. Not a production substrate -- no TLS,
//! no auth, no persistence across restarts, no reconnection handling --
//! see `iroh_substrate.rs`/`atproto_cid.rs` for the two real backends
//! this workspace ships. It exists so `dmml-runtime`'s trait usage can
//! be sanity-checked against a genuinely separate process/connection
//! without paying iroh's build cost to do it.
//!
//! Protocol: one JSON [`Request`] per line, one JSON [`Response`] per
//! line, one request per connection (the client opens a fresh
//! `TcpStream` for every `Substrate`/`AppendSubstrate` call and closes
//! it after reading the reply). `serde_json::to_string`'s compact
//! output never contains a literal newline -- string values with an
//! embedded `\n` are escaped, not broken across lines -- so newline
//! framing is safe here without a length prefix.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use oxigraph::model::{NamedNode, NamedOrBlankNode};

use dmml_runtime::graph::{parse_nquads, Commit, ConsumeRef, FactRef};
use dmml_runtime::substrate::{
    AppendSubstrate, Assertion, CommitReceipt, CommitRecord, RetractionStatus, Substrate,
};

/// An opaque author identity -- same role as `mock::MockIdentity`, a
/// label with no credential material, since there's no real admission
/// gate to authenticate to here either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketIdentity(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketSubstrateError(pub String);

impl std::fmt::Display for SocketSubstrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SocketSubstrateError {}

impl From<std::io::Error> for SocketSubstrateError {
    fn from(err: std::io::Error) -> Self {
        SocketSubstrateError(format!("socket io failed: {err}"))
    }
}

impl From<serde_json::Error> for SocketSubstrateError {
    fn from(err: serde_json::Error) -> Self {
        SocketSubstrateError(format!("request/response encoding failed: {err}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCommit {
    cid: String,
    author: SocketIdentity,
    commit: Commit,
}

#[derive(Debug, Serialize, Deserialize)]
enum Request {
    Append {
        author: SocketIdentity,
        commit: Commit,
    },
    GetCommit {
        cid: String,
    },
    AllCommits,
}

#[derive(Debug, Serialize, Deserialize)]
enum Response {
    Appended { cid: String },
    Commit(Option<StoredCommit>),
    AllCommits(Vec<StoredCommit>),
    Error(String),
}

/// The shared, in-memory log a [`SocketServer`] serves -- the same
/// shape as `mock::MockAppendSubstrate`'s own state, just reachable
/// over a real socket instead of a direct method call.
#[derive(Default)]
struct ServerState {
    log: RwLock<Vec<StoredCommit>>,
    next_cid: RwLock<u64>,
}

impl ServerState {
    fn fresh_cid(&self) -> String {
        let mut n = self.next_cid.write().expect("server lock poisoned");
        let cid = format!("socket-cid-{n}");
        *n += 1;
        cid
    }

    fn handle(&self, req: Request) -> Response {
        match req {
            Request::Append { author, commit } => {
                let cid = self.fresh_cid();
                self.log
                    .write()
                    .expect("server lock poisoned")
                    .push(StoredCommit {
                        cid: cid.clone(),
                        author,
                        commit,
                    });
                Response::Appended { cid }
            }
            Request::GetCommit { cid } => {
                let log = self.log.read().expect("server lock poisoned");
                Response::Commit(log.iter().find(|c| c.cid == cid).cloned())
            }
            Request::AllCommits => {
                let log = self.log.read().expect("server lock poisoned");
                Response::AllCommits(log.clone())
            }
        }
    }
}

/// A running server for one world's worth of commits. Hold this alive
/// for as long as any [`SocketAppendSubstrate`] client needs to reach
/// it -- dropping it (or the task `serve` was spawned on) closes the
/// listener.
pub struct SocketServer {
    pub local_addr: SocketAddr,
}

impl SocketServer {
    /// Bind a listener on `addr` (use `127.0.0.1:0` to let the OS pick
    /// a free local port -- the actual bound address is on the
    /// returned value's `local_addr`) and spawn its accept loop on the
    /// current tokio runtime. Every accepted connection is handled on
    /// its own spawned task, so multiple clients/authors can be
    /// in-flight concurrently, same as `iroh_substrate`'s real
    /// multi-writer shape.
    pub async fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        let state = Arc::new(ServerState::default());
        tokio::spawn(Self::accept_loop(listener, state));
        Ok(Self { local_addr })
    }

    async fn accept_loop(listener: TcpListener, state: Arc<ServerState>) {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                let _ = Self::handle_connection(stream, state).await;
            });
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        state: Arc<ServerState>,
    ) -> Result<(), SocketSubstrateError> {
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let response = match serde_json::from_str::<Request>(line.trim_end()) {
            Ok(req) => state.handle(req),
            Err(e) => Response::Error(format!("malformed request: {e}")),
        };
        let mut encoded = serde_json::to_string(&response)?;
        encoded.push('\n');
        write_half.write_all(encoded.as_bytes()).await?;
        Ok(())
    }
}

/// A client bound to one sovereignty root (`owner`/`namespace`), talking
/// to a [`SocketServer`] over `addr`. Every `Substrate`/`AppendSubstrate`
/// call opens its own connection -- see this module's doc comment for
/// why that's an acceptable simplification here.
pub struct SocketAppendSubstrate {
    owner: SocketIdentity,
    namespace: String,
    addr: SocketAddr,
}

impl SocketAppendSubstrate {
    pub fn new(owner: SocketIdentity, namespace: impl Into<String>, addr: SocketAddr) -> Self {
        Self {
            owner,
            namespace: namespace.into(),
            addr,
        }
    }

    async fn call(&self, req: &Request) -> Result<Response, SocketSubstrateError> {
        let stream = TcpStream::connect(self.addr).await?;
        let (read_half, mut write_half) = stream.into_split();
        let mut encoded = serde_json::to_string(req)?;
        encoded.push('\n');
        write_half.write_all(encoded.as_bytes()).await?;
        write_half.shutdown().await?;

        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line.is_empty() {
            return Err(SocketSubstrateError(
                "server closed connection with no reply".into(),
            ));
        }
        Ok(serde_json::from_str(line.trim_end())?)
    }

    async fn all_commits(&self) -> Result<Vec<StoredCommit>, SocketSubstrateError> {
        match self.call(&Request::AllCommits).await? {
            Response::AllCommits(entries) => Ok(entries),
            Response::Error(e) => Err(SocketSubstrateError(e)),
            other => Err(SocketSubstrateError(format!(
                "unexpected reply to AllCommits: {other:?}"
            ))),
        }
    }

    /// Mirror of `mock.rs`/`iroh_substrate.rs`'s identical `cites()`
    /// matching semantics -- kept in sync by hand across all three,
    /// same as those two already are with each other.
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

impl Substrate for SocketAppendSubstrate {
    type Identity = SocketIdentity;
    type Error = SocketSubstrateError;

    fn name(&self) -> &'static str {
        "socket-append"
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
        let cid = cid.to_string();
        async move {
            match self
                .call(&Request::GetCommit { cid: cid.clone() })
                .await?
            {
                Response::Commit(entry) => Ok(entry.map(|c| CommitRecord {
                    cid: c.cid,
                    commit: c.commit,
                })),
                Response::Error(e) => Err(SocketSubstrateError(e)),
                other => Err(SocketSubstrateError(format!(
                    "unexpected reply to GetCommit: {other:?}"
                ))),
            }
        }
    }

    fn resolve_fact(
        &self,
        fact: &FactRef,
    ) -> impl Future<Output = Result<RetractionStatus, Self::Error>> + Send {
        let fact = fact.clone();
        async move {
            let by: Vec<String> = self
                .all_commits()
                .await?
                .into_iter()
                .filter(|c| {
                    c.commit.consumes.iter().any(|r| match r {
                        ConsumeRef::Fact(consumed) => Self::cites(consumed, &fact),
                        ConsumeRef::Strong(_) => false,
                    })
                })
                .map(|c| c.cid)
                .collect();
            Ok(if by.is_empty() {
                RetractionStatus::Live
            } else {
                RetractionStatus::Retracted { by }
            })
        }
    }

    fn assertions(
        &self,
        subject: &NamedNode,
        predicate: &NamedNode,
    ) -> impl Future<Output = Result<Vec<Assertion>, Self::Error>> + Send {
        let subject = subject.clone();
        let predicate = predicate.clone();
        async move {
            let mut out = Vec::new();
            for c in self.all_commits().await? {
                let quads = parse_nquads(&c.commit.produces).map_err(|e| {
                    SocketSubstrateError(format!(
                        "n-quads parse failed for commit {}: {e}",
                        c.cid
                    ))
                })?;
                for q in quads {
                    let subject_matches = match &q.subject {
                        NamedOrBlankNode::NamedNode(n) => n == &subject,
                        NamedOrBlankNode::BlankNode(_) => false,
                    };
                    if subject_matches && q.predicate == predicate {
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
}

impl AppendSubstrate for SocketAppendSubstrate {
    fn append_commit(
        &self,
        author: &Self::Identity,
        commit: &Commit,
    ) -> impl Future<Output = Result<CommitReceipt, Self::Error>> + Send {
        let author = author.clone();
        let commit = commit.clone();
        async move {
            match self.call(&Request::Append { author, commit }).await? {
                Response::Appended { cid } => Ok(CommitReceipt { cid }),
                Response::Error(e) => Err(SocketSubstrateError(e)),
                other => Err(SocketSubstrateError(format!(
                    "unexpected reply to Append: {other:?}"
                ))),
            }
        }
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
            created_at: "2026-08-29T00:00:00Z".to_string(),
        }
    }

    async fn server() -> SocketServer {
        SocketServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .expect("bind failed")
    }

    #[tokio::test]
    async fn a_bare_produces_never_retracts_anything_over_a_real_socket() {
        let srv = server().await;
        let sub = SocketAppendSubstrate::new(
            SocketIdentity("did:example:alice".into()),
            "world/1",
            srv.local_addr,
        );
        let author = SocketIdentity("author:device-a".into());

        sub.append_commit(
            &author,
            &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."),
        )
        .await
        .unwrap();

        let fact = FactRef {
            commit: StrongRef {
                uri: "at://did:example:alice/x/rkey1".into(),
                cid: "socket-cid-0".into(),
            },
            subject: "sky/1".into(),
            predicate: "origin".into(),
            object: None,
        };
        assert_eq!(
            sub.resolve_fact(&fact).await.unwrap(),
            RetractionStatus::Live
        );
    }

    #[tokio::test]
    async fn two_independent_connections_consuming_the_same_base_is_the_real_conflict_signature() {
        let srv = server().await;
        // Two entirely separate client values, each opening its own
        // fresh TCP connections per call -- the thing `mock.rs` cannot
        // exercise at all, since it only ever has one in-process value.
        let device_a = SocketAppendSubstrate::new(
            SocketIdentity("did:example:alice".into()),
            "world/1",
            srv.local_addr,
        );
        let device_b = SocketAppendSubstrate::new(
            SocketIdentity("did:example:alice".into()),
            "world/1",
            srv.local_addr,
        );
        let author_a = SocketIdentity("author:device-a".into());
        let author_b = SocketIdentity("author:device-b".into());

        let base_receipt = device_a
            .append_commit(
                &author_a,
                &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."),
            )
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

        device_a
            .append_commit(
                &author_a,
                &commit(
                    vec![ConsumeRef::Fact(base.clone())],
                    "<x:sky/1> <x:origin> \"duskweave\" .",
                ),
            )
            .await
            .unwrap();
        device_b
            .append_commit(
                &author_b,
                &commit(
                    vec![ConsumeRef::Fact(base.clone())],
                    "<x:sky/1> <x:origin> \"moonwoven\" .",
                ),
            )
            .await
            .unwrap();

        match device_a.resolve_fact(&base).await.unwrap() {
            RetractionStatus::Retracted { by } => {
                assert_eq!(by.len(), 2, "both concurrent consumers should show up: {by:?}");
            }
            RetractionStatus::Live => panic!("a twice-consumed base must not read as Live"),
        }
    }

    #[tokio::test]
    async fn get_commit_round_trips_over_a_fresh_connection() {
        let srv = server().await;
        let sub = SocketAppendSubstrate::new(
            SocketIdentity("did:example:alice".into()),
            "world/1",
            srv.local_addr,
        );
        let author = SocketIdentity("author:device-a".into());

        let receipt = sub
            .append_commit(
                &author,
                &commit(vec![], "<x:sky/1> <x:origin> \"sunfire\" ."),
            )
            .await
            .unwrap();

        let fetched = sub.get_commit(&receipt.cid).await.unwrap();
        assert!(fetched.is_some(), "just-appended commit must read back");
        assert_eq!(fetched.unwrap().cid, receipt.cid);

        assert!(sub.get_commit("no-such-cid").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn assertions_returns_every_independent_production_over_sockets() {
        let srv = server().await;
        let sub = SocketAppendSubstrate::new(
            SocketIdentity("did:example:alice".into()),
            "world/1",
            srv.local_addr,
        );
        let a = SocketIdentity("author:helios".into());
        let b = SocketIdentity("author:selene".into());

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
