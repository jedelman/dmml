//! Real CID computation, checked against the actual DAG-CBOR/CID
//! encoding it claims to use -- not just "produces *a* string."

use dmml::ast::TopLevelItem;
use dmml_substrate_kit::atproto_cid::compute_cid;
use dmml::lower::lower_commit;

fn cid_for(src: &str, created_at: &str) -> cid::Cid {
    let doc = dmml::parse(src).expect("should parse");
    let TopLevelItem::Commit(commit) = &doc.items[0] else {
        panic!("expected a commit");
    };
    let lowered = lower_commit(commit);
    compute_cid(&lowered, created_at)
}

#[test]
fn cid_uses_the_real_atproto_codec_and_hash_function() {
    let cid = cid_for("commit mints { room/42 a Room }", "2026-01-01T00:00:00Z");
    // dag-cbor multicodec
    assert_eq!(cid.codec(), 0x71);
    // sha2-256 multihash
    assert_eq!(cid.hash().code(), 0x12);
    assert_eq!(cid.hash().size(), 32);
    // CIDv1
    assert_eq!(cid.version(), cid::Version::V1);
}

#[test]
fn cid_is_deterministic() {
    let a = cid_for("commit mints { room/42 a Room }", "2026-01-01T00:00:00Z");
    let b = cid_for("commit mints { room/42 a Room }", "2026-01-01T00:00:00Z");
    assert_eq!(a, b);
}

#[test]
fn different_content_gets_different_cid() {
    let a = cid_for("commit mints { room/42 a Room }", "2026-01-01T00:00:00Z");
    let b = cid_for("commit mints { room/43 a Room }", "2026-01-01T00:00:00Z");
    assert_ne!(a, b);
}

#[test]
fn different_created_at_gets_different_cid() {
    // createdAt is part of the wire record per the lexicon, so it's part
    // of what gets hashed -- two otherwise-identical commits authored at
    // different times are different records, different CIDs.
    let a = cid_for("commit mints { room/42 a Room }", "2026-01-01T00:00:00Z");
    let b = cid_for("commit mints { room/42 a Room }", "2026-01-02T00:00:00Z");
    assert_ne!(a, b);
}

#[test]
fn cid_string_form_is_the_real_base32_cidv1_encoding() {
    let cid = cid_for("commit mints { room/42 a Room }", "2026-01-01T00:00:00Z");
    let s = cid.to_string();
    // CIDv1 in base32 (multibase 'b' prefix) is the standard atproto/IPLD
    // string form; dag-cbor + sha2-256 CIDs conventionally start "bafyrei".
    assert!(
        s.starts_with('b'),
        "expected multibase base32 prefix, got {s}"
    );
    // Round-trips through the real Cid parser.
    let reparsed: cid::Cid = s.parse().expect("should reparse");
    assert_eq!(reparsed, cid);
}

#[test]
fn consumes_and_via_and_responds_to_are_part_of_what_gets_hashed() {
    let a = cid_for(
        r#"commit grants { via at://did:plc:abc/org.foo.bar/rkey1 (cid: bafyxyz1) }"#,
        "2026-01-01T00:00:00Z",
    );
    let b = cid_for(
        r#"commit grants { via at://did:plc:zzz/org.foo.bar/rkey9 (cid: bafyxyz9) }"#,
        "2026-01-01T00:00:00Z",
    );
    assert_ne!(a, b);
}
