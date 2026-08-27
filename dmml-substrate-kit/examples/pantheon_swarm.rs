//! The `pantheon.rs` scenario, for real, over live `iroh-docs` storage.
//!
//! Same finding as written-world's original `pantheon.rs` -- three
//! uncoordinated origins for one node coexist rather than overwriting each
//! other, and the only real conflict shape is two commits, unaware of each
//! other, `consumes`-citing the identical prior fact, resolved via a
//! `disputes` commit that names both rivals rather than picking a winner --
//! but proven here against real content-addressed storage and real
//! author-partitioned writes (`IrohAppendSubstrate`), not an in-memory mock.
//!
//! Scoped deliberately to one process: every "god" here is a distinct
//! `AuthorId` writing into the SAME local `Doc`, not a separate network
//! node. iroh-docs' `(namespace, author, key)` partitioning is exactly what
//! makes that a faithful proof of the concurrent-writer story --
//! `dmml/ARCHITECTURE.md`'s hot-path design turns on author partitioning,
//! not on how many OS processes happen to hold the authors. Real multi-node
//! network sync (`doc.share()`/`api.import()` between separate `Endpoint`s)
//! is the natural next step, not yet built here -- this proves the storage
//! and conflict-detection semantics are real first, per this project's own
//! DMML-first discipline. `Endpoint::empty()` is used specifically because
//! its own doc comment guarantees "no address lookup services, and
//! RelayMode::Disabled" -- this example never dials out, so it shouldn't
//! even attempt to.

use dmml_runtime::graph::{Commit, ConsumeRef, FactRef, StrongRef};
use dmml_runtime::substrate::{AppendSubstrate, RetractionStatus, Substrate};
use dmml_substrate_kit::iroh_substrate::IrohAppendSubstrate;
use iroh::endpoint::Builder as EndpointBuilder;
use iroh_blobs::store::mem::MemStore;
use iroh_docs::protocol::Docs;
use iroh_docs::store::Query;
use iroh_gossip::net::Gossip;
use n0_future::StreamExt;
use oxigraph::model::NamedNode;

fn commit(consumes: Vec<ConsumeRef>, predicate: &str, produces: String) -> Commit {
    Commit {
        consumes,
        produces,
        predicate: predicate.to_string(),
        via: None,
        responds_to: None,
        created_at: "2026-08-27T00:00:00Z".to_string(),
    }
}

/// A `FactRef` citing exactly one triple a prior commit produced, matching
/// `mock.rs`'s own test construction pattern.
fn fact_ref(cid: &str, subject: &str, predicate: &str) -> FactRef {
    FactRef {
        commit: StrongRef {
            uri: format!("iroh://pantheon-swarm/{cid}"),
            cid: cid.to_string(),
        },
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon swarm: real iroh-docs storage, no network dialing ==\n");

    // `Builder::empty()` has no address lookup services and RelayMode::
    // Disabled by construction (its own doc comment), but also no crypto
    // provider set (bind() requires one) -- set it explicitly rather than
    // reaching for a preset, since every preset also wires up real internet
    // discovery/relay services this process has no reason to touch: nothing
    // below ever dials out (no share()/import()), every "god" here writes
    // into the same local Doc. The N0-preset version of this hung
    // indefinitely in this sandbox trying to reach real internet
    // discovery/relay infrastructure -- confirmed by testing, not assumed.
    let endpoint = EndpointBuilder::empty()
        .crypto_provider(std::sync::Arc::new(rustls::crypto::ring::default_provider()))
        .bind()
        .await?;
    let blobs = MemStore::default();
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let docs = Docs::memory()
        .spawn(endpoint.clone(), (*blobs).clone(), gossip.clone())
        .await?;
    let api = docs.api();

    let helios = api.author_create().await?;
    let selene = api.author_create().await?;
    let eos = api.author_create().await?;
    let nyx = api.author_create().await?;
    let pantheon_council = api.author_create().await?;

    let doc = api.create().await?;
    println!("doc namespace: {}\n", doc.id());

    // Keep our own `doc` handle (Doc is Clone) for the final summary listing
    // below, since the substrate owns its own clone internally.
    let substrate =
        IrohAppendSubstrate::new(pantheon_council, "pantheon-swarm".to_string(), doc.clone(), (*blobs).clone());

    // ---- Step 1: non-destructive multiplicity (the original pantheon.rs finding) ----
    println!("-- three uncoordinated origins for sky/1 --");
    let helios_receipt = substrate
        .append_commit(
            &helios,
            &commit(
                vec![],
                "asserts",
                "<x:sky/1> <x:origin> \"sunfire\" .".to_string(),
            ),
        )
        .await?;
    println!("helios  -> {} : sunfire", helios_receipt.cid);

    let selene_receipt = substrate
        .append_commit(
            &selene,
            &commit(
                vec![],
                "asserts",
                "<x:sky/1> <x:origin> \"moonwoven\" .".to_string(),
            ),
        )
        .await?;
    println!("selene  -> {} : moonwoven", selene_receipt.cid);

    let eos_receipt = substrate
        .append_commit(
            &eos,
            &commit(
                vec![],
                "asserts",
                "<x:sky/1> <x:origin> \"rosefingered\" .".to_string(),
            ),
        )
        .await?;
    println!("eos     -> {} : rosefingered", eos_receipt.cid);

    let subject = NamedNode::new("x:sky/1")?;
    let predicate = NamedNode::new("x:origin")?;
    let all = substrate.assertions(&subject, &predicate).await?;
    println!(
        "\nreal read-back: {} independent assertions coexist for (sky/1, origin) -- \
         a bare produces never overwrites (proven against real storage, not asserted)",
        all.len()
    );
    assert_eq!(all.len(), 3, "all three origins must coexist");

    // ---- Step 2: the real conflict ----
    println!("\n-- two uncoordinated syntheses, both citing helios's origin as their base --");
    let base = fact_ref(&helios_receipt.cid, "sky/1", "origin");

    let nyx_receipt = substrate
        .append_commit(
            &nyx,
            &commit(
                vec![ConsumeRef::Fact(base.clone())],
                "weaves",
                "<x:sky/1> <x:origin> \"duskweave\" .".to_string(),
            ),
        )
        .await?;
    println!(
        "nyx              -> {} : duskweave (consumes helios)",
        nyx_receipt.cid
    );

    let rival_receipt = substrate
        .append_commit(
            &pantheon_council,
            &commit(
                vec![ConsumeRef::Fact(base.clone())],
                "weaves",
                "<x:sky/1> <x:origin> \"starforge\" .".to_string(),
            ),
        )
        .await?;
    println!(
        "pantheon_council -> {} : starforge (consumes helios, unaware of nyx)",
        rival_receipt.cid
    );

    match substrate.resolve_fact(&base).await? {
        RetractionStatus::Retracted { by } => {
            println!(
                "\nreal conflict detected: helios's origin is Retracted, by {} commits: {:?}",
                by.len(),
                by
            );
            assert_eq!(by.len(), 2, "both concurrent consumers must show up");
        }
        RetractionStatus::Live => panic!("a twice-consumed base must not read as Live"),
    }

    // ---- Step 3: resolution via disputes, never arbitration ----
    println!("\n-- resolving via disputes: neither synthesis wins --");
    let disputes_consumes = vec![
        ConsumeRef::Fact(fact_ref(&nyx_receipt.cid, "sky/1", "origin")),
        ConsumeRef::Fact(fact_ref(&rival_receipt.cid, "sky/1", "origin")),
    ];
    let disputes_receipt = substrate
        .append_commit(
            &pantheon_council,
            &commit(
                disputes_consumes,
                "disputes",
                "<x:sky/1> <x:disputedOrigin> \"duskweave-vs-starforge, unresolved\" .".to_string(),
            ),
        )
        .await?;
    println!(
        "pantheon_council -> {} : disputes both duskweave and starforge, picks neither",
        disputes_receipt.cid
    );

    // ---- Summary: every real entry now in the doc ----
    println!("\n-- final state: every real entry in the doc --");
    let stream = doc.get_many(Query::all()).await?;
    tokio::pin!(stream);
    let mut count = 0usize;
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        count += 1;
        println!(
            "  entry {:>2}: author={} key={}",
            count,
            entry.author(),
            String::from_utf8_lossy(entry.key())
        );
    }
    println!(
        "\n{count} real entries total, all real content-addressed writes, zero network dials."
    );

    Ok(())
}
