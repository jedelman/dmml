#!/usr/bin/env python3
"""dmml-agent-nucleus reference harness.

Stdlib only, on purpose -- "any agent with git in their terminal" should
mean literally that, not "any agent with git and a working Rust toolchain
and 15GB of disk for iroh-blobs." This is the minimal thing, not the fast
thing. It mirrors, in Python, exactly the shape `dmml-runtime`'s `Commit`
struct uses in Rust (see GRAMMAR.md) -- nothing here reinterprets DMML,
it's the same five fields, the same StrongRef/FactRef shapes, checkpointed
against a real atproto PDS the same way every checkpoint_*.py script in
the sibling dmml repo's dmml-substrate-kit/ does it tonight.

Read this file once. Then change it, or don't, or throw it out and write
your own. See README.md.
"""

import hashlib
import json
import os
import time
import urllib.request
from dataclasses import dataclass, field, asdict
from typing import Optional


# ---------------------------------------------------------------------------
# Identity. Read from YOUR OWN environment. Never hardcode someone else's.
# See README.md's "use your own identity" section -- this is not optional.
# ---------------------------------------------------------------------------

PDS = os.environ.get("DMML_PDS", "")           # e.g. https://your.pds.host
DID = os.environ.get("DMML_DID", "")            # e.g. did:plc:yourselfyourself
APP_PASSWORD = os.environ.get("DMML_APP_PASSWORD", "")
COLLECTION = os.environ.get(
    "DMML_COLLECTION", "org.jason-edelman.writtenworld.commit"
)
# ^ Reusing this NSID is fine -- it's a real, working, tested record shape,
# and any DID can host records under any collection NSID on their own PDS.
# It is NOT a claim of affiliation with written-world. Mint your own NSID
# later if this grows past experimentation.


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
    """A citation into something DMML's grammar will never execute or even
    look inside: a code repo, a script, a commit on a knot server, whatever.
    `cid_or_sha` is ideally a real atproto CID (e.g. a tangled.sh
    sh.tangled.git.refUpdate record's cid, resolvable the same way any
    other citation is) -- if you only have a raw git commit SHA, that's
    still a valid opaque string for `apply_commit`'s existence check
    (it does plain string comparison, nothing atproto-specific is
    enforced at the type level), just say so honestly wherever you use it.
    """
    if note:
        print(f"[off-protocol link] {note}: {uri} @ {cid_or_sha}")
    return StrongRef(uri=uri, cid=cid_or_sha)


# ---------------------------------------------------------------------------
# Checkpointing. Same pattern as every checkpoint_*.py used tonight:
# create a session, POST the record, get back the real {uri, cid}.
# ---------------------------------------------------------------------------

def _require_identity():
    missing = [n for n, v in [("DMML_PDS", PDS), ("DMML_DID", DID), ("DMML_APP_PASSWORD", APP_PASSWORD)] if not v]
    if missing:
        raise RuntimeError(
            f"missing env var(s): {', '.join(missing)} -- set these to YOUR OWN "
            "PDS/DID/app-password before checkpointing. Never copy another "
            "identity's credentials into this file or your environment."
        )


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
    """Fetch a real record back from the network before you trust a
    citation to it. This is the spot-check every real checkpoint run
    tonight did before calling itself done -- do this before citing
    anyone else's work, including your own earlier commits."""
    parts = ref.uri.replace("at://", "").split("/")
    did, collection, rkey = parts[0], parts[1], parts[2]
    # NOTE: resolves against THIS harness's own PDS var for simplicity;
    # cross-PDS resolution needs the target's actual PDS host, which you
    # get by resolving the DID document first. Left as an exercise --
    # this reference harness does not pretend to be complete.
    url = f"{PDS}/xrpc/com.atproto.repo.getRecord?repo={did}&collection={collection}&rkey={rkey}"
    with urllib.request.urlopen(url) as resp:
        record = json.loads(resp.read())
    if prompt:
        print(f"[verify] {prompt}: {json.dumps(record['value'])[:200]}")
    return record


def local_reference(commit: Commit) -> str:
    """A LOCAL-ONLY placeholder id for a commit that hasn't been
    checkpointed yet -- sha256 over its canonical JSON. This is NOT an
    atproto CID and is not resolvable by anyone else. Never cite this in
    a real `consumes` entry; only real, checkpointed {uri, cid} pairs
    belong there. Use this only to track your own not-yet-published work
    locally before checkpointing it."""
    canonical = json.dumps(asdict(commit), sort_keys=True)
    return hashlib.sha256(canonical.encode()).hexdigest()


if __name__ == "__main__":
    # A two-commit demo: mint locally, checkpoint the first, cite it for
    # real from the second, verify the citation. Requires your own
    # DMML_PDS / DMML_DID / DMML_APP_PASSWORD to actually checkpoint --
    # without them, this just prints the local shape and stops there.
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
