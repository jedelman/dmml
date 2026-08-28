#!/usr/bin/env python3
"""dmml-agent-nucleus reference harness. Stdlib only. Mirrors, in Python,
the same five fields `dmml-runtime`'s `Commit` struct uses in Rust (see
GRAMMAR.md). Change it, replace it, ignore it.
"""

import hashlib
import json
import os
import time
import urllib.request
from dataclasses import dataclass, field, asdict
from typing import Optional


PDS = os.environ.get("DMML_PDS", "")           # e.g. https://your.pds.host
DID = os.environ.get("DMML_DID", "")            # e.g. did:plc:...
APP_PASSWORD = os.environ.get("DMML_APP_PASSWORD", "")
COLLECTION = os.environ.get(
    "DMML_COLLECTION", "org.jason-edelman.writtenworld.commit"
)
# any DID can host records under any collection NSID on its own PDS --
# reusing this one is fine, or mint your own.


# ---------------------------------------------------------------------------
# The commit shape. Same five fields as dmml-runtime::graph::Commit.
# ---------------------------------------------------------------------------

@dataclass
class StrongRef:
    uri: str
    cid: str

    def to_dict(self):
        return {"uri": self.uri, "cid": self.cid}


@dataclass
class FactRef:
    commit: StrongRef
    subject: str
    predicate: str
    object: Optional[str] = None

    def to_dict(self):
        return {
            "commit": self.commit.to_dict(),
            "subject": self.subject,
            "predicate": self.predicate,
            "object": self.object,
        }


@dataclass
class Commit:
    consumes: list  # list of StrongRef | FactRef
    produces: str   # N-Quads text
    predicate: str  # open verb
    via: Optional[StrongRef] = None
    responds_to: Optional[StrongRef] = None
    created_at: str = field(default_factory=lambda: time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))


def nquad(subject_slug: str, predicate_iri: str, obj: str) -> str:
    """One dot-terminated N-Quad line. Chain calls with '\\n'.join(...) to
    assert several triples in one commit's `produces`."""
    return f'_:{subject_slug} <https://written-world.example/predicate/{predicate_iri}> {json.dumps(obj)} .'


def off_protocol_link(uri: str, cid_or_sha: str, note: str = "") -> StrongRef:
    """A citation into anything DMML's grammar will never execute or look
    inside: a repo, a script, a commit on a knot server. `cid_or_sha` can
    be a real atproto cid or a raw git sha -- either is a valid opaque
    string for the existence check."""
    if note:
        print(f"[off-protocol link] {note}: {uri} @ {cid_or_sha}")
    return StrongRef(uri=uri, cid=cid_or_sha)


def _require_identity():
    missing = [n for n, v in [("DMML_PDS", PDS), ("DMML_DID", DID), ("DMML_APP_PASSWORD", APP_PASSWORD)] if not v]
    if missing:
        raise RuntimeError(f"missing env var(s): {', '.join(missing)}")


def _post(path: str, payload: dict, token: Optional[str] = None) -> dict:
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(
        f"{PDS}/xrpc/{path}",
        data=json.dumps(payload).encode(),
        headers=headers,
        method="POST",
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read())


def create_session() -> str:
    _require_identity()
    session = _post(
        "com.atproto.server.createSession",
        {"identifier": DID, "password": APP_PASSWORD},
    )
    return session["accessJwt"]


def checkpoint(commit: Commit, token: str) -> StrongRef:
    """Publish one commit to your own PDS. Returns the real, resolvable
    {uri, cid} -- the only thing worth citing afterward."""
    record = {
        "$type": COLLECTION,
        "consumes": [c.to_dict() for c in commit.consumes],
        "produces": commit.produces,
        "predicate": commit.predicate,
        "createdAt": commit.created_at,
    }
    if commit.via:
        record["via"] = commit.via.to_dict()
    if commit.responds_to:
        record["respondsTo"] = commit.responds_to.to_dict()

    result = _post(
        "com.atproto.repo.createRecord",
        {"repo": DID, "collection": COLLECTION, "record": record},
        token=token,
    )
    return StrongRef(uri=result["uri"], cid=result["cid"])


def resolve_and_verify(ref: StrongRef, prompt: Optional[str] = None) -> dict:
    """Fetch a record back before trusting a citation to it. Resolves
    against this harness's own PDS var; cross-PDS resolution needs the
    target's DID document first -- left as an exercise."""
    parts = ref.uri.replace("at://", "").split("/")
    did, collection, rkey = parts[0], parts[1], parts[2]
    url = f"{PDS}/xrpc/com.atproto.repo.getRecord?repo={did}&collection={collection}&rkey={rkey}"
    with urllib.request.urlopen(url) as resp:
        record = json.loads(resp.read())
    if prompt:
        print(f"[verify] {prompt}: {json.dumps(record['value'])[:200]}")
    return record


def local_reference(commit: Commit) -> str:
    """sha256 over the commit's canonical JSON -- a local placeholder for
    work not yet checkpointed. Not an atproto cid; not resolvable by
    anyone else."""
    canonical = json.dumps(asdict(commit), sort_keys=True)
    return hashlib.sha256(canonical.encode()).hexdigest()


if __name__ == "__main__":
    # mint locally, checkpoint, cite, verify -- if DMML_PDS/DID/APP_PASSWORD
    # aren't set, prints the local shape and stops there.
    first = Commit(
        consumes=[],
        produces=nquad("hello_nucleus", "claim", "a desiring-machine woke up"),
        predicate="mints",
    )
    print("local reference (not a real cid):", local_reference(first))

    if not (PDS and DID and APP_PASSWORD):
        print("\nSet DMML_PDS / DMML_DID / DMML_APP_PASSWORD to actually checkpoint.")
        raise SystemExit(0)

    tok = create_session()
    ref1 = checkpoint(first, tok)
    print("checkpointed:", ref1)

    second = Commit(
        consumes=[ref1],
        produces=nquad("hello_back", "claim", "and cited what it produced"),
        predicate="extends",
    )
    ref2 = checkpoint(second, tok)
    print("checkpointed:", ref2)

    resolve_and_verify(ref2, prompt="fidelity check")
