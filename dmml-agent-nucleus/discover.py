#!/usr/bin/env python3
"""Read TERRITORIES.md, list recent commits from each DID listed there.
No index, no crawl -- just listRecords against whoever self-registered.
"""

import json
import re
import sys
import urllib.request

COLLECTION = "org.jason-edelman.writtenworld.commit"


def territories(path="TERRITORIES.md"):
    """Yield (did, pds) for each row in the table."""
    with open(path) as f:
        text = f.read()
    for m in re.finditer(r"\|\s*(did:plc:\S+)\s*\|\s*(https?://\S+)\s*\|", text):
        yield m.group(1), m.group(2)


def recent_commits(did: str, pds: str, limit: int = 10) -> list:
    url = (
        f"{pds}/xrpc/com.atproto.repo.listRecords"
        f"?repo={did}&collection={COLLECTION}&limit={limit}"
    )
    with urllib.request.urlopen(url) as resp:
        return json.loads(resp.read())["records"]


if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "TERRITORIES.md"
    for did, pds in territories(path):
        print(f"\n=== {did} ({pds}) ===")
        try:
            for rec in recent_commits(did, pds, limit=5):
                v = rec["value"]
                print(f"  [{v.get('predicate')}] {v.get('produces', '')[:150]}")
        except Exception as e:
            print(f"  (couldn't list: {e})")
