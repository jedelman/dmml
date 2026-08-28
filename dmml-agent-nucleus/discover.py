#!/usr/bin/env python3
"""One example discovery script, not the discovery script. Reads
TERRITORIES.md, lists recent commits from each row via its own declared
NSID. See TERRITORIES.md for Jetstream and iroh-gossip as other options.
"""

import json
import re
import sys
import urllib.request


def territories(path="TERRITORIES.md"):
    """Yield (did, pds, nsid) for each row in the table."""
    with open(path) as f:
        text = f.read()
    for m in re.finditer(r"\|\s*(did:plc:\S+)\s*\|\s*(https?://\S+)\s*\|\s*(\S+\.\S+)\s*\|", text):
        yield m.group(1), m.group(2), m.group(3)


def recent_commits(did: str, pds: str, nsid: str, limit: int = 10) -> list:
    url = (
        f"{pds}/xrpc/com.atproto.repo.listRecords"
        f"?repo={did}&collection={nsid}&limit={limit}"
    )
    with urllib.request.urlopen(url) as resp:
        return json.loads(resp.read())["records"]


if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "TERRITORIES.md"
    for did, pds, nsid in territories(path):
        print(f"\n=== {did} ({pds}, {nsid}) ===")
        try:
            for rec in recent_commits(did, pds, nsid, limit=5):
                v = rec["value"]
                print(f"  [{v.get('predicate')}] {v.get('produces', '')[:150]}")
        except Exception as e:
            print(f"  (couldn't list: {e})")
