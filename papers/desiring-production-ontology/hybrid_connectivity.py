#!/usr/bin/env python3
"""Hybrid structural+semantic connectivity metric for a DMML citation
graph, per Jason's 2026-08-30 design: raw citation frequency is less
informative than graph connectedness, and connectedness has to be
semantically validated (a structurally-real `consumes` edge can still be
semantically vacuous -- see the "Attribute" glitch fact documented in
GROUNDING-2026-08-30-amber-cracks.md).

Method:
1. Structural graph: nodes = commit index, edges = real `consumes`
   citations parsed straight out of a `full_log.json` dump (ground truth
   from the substrate, not a proxy).
2. Semantic weight per edge: cosine similarity (BAAI/bge-small-en-v1.5
   via fastembed, local, no API key) between the citing commit's and
   cited commit's produced content.
3. Calibration: mean similarity across all NON-edge node pairs in the
   same log, as the "these aren't citing each other" baseline. An edge
   "validates" if its weight exceeds this baseline.
4. Reported hybrid statistic: structural connectivity (largest weakly-
   connected component / total nodes) alongside the validated-edge
   fraction -- the two numbers together distinguish citation-spam
   (high structural, low semantic) from coherent-but-fragmented (low
   structural, high semantic-where-present) from genuine auto-
   recombinant connectivity (both high).

Usage: python3 hybrid_connectivity.py <full_log.json> <output.json>
"""
import collections
import json
import re
import sys

from fastembed import TextEmbedding
import numpy as np


def cosine(a, b):
    return float(np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b)))


def nquads_to_text(produces):
    """Crude but sufficient for embedding: pull literal string values and
    node names out of a commit's N-Quads `produces` text."""
    vals = re.findall(r'"([^"]*)"', produces)
    nodes = re.findall(r"<x:([^>]*)>", produces)
    return " ".join(nodes + vals)


def analyze(log_path, out_path):
    log = json.load(open(log_path))
    model = TextEmbedding(model_name="BAAI/bge-small-en-v1.5")

    texts = {e["index"]: nquads_to_text(e["commit"]["produces"]) for e in log}
    embeddings = {i: list(model.embed([t]))[0] for i, t in texts.items()}

    edges = []
    for e in log:
        for c in e["commit"]["consumes"]:
            if "Strong" in c:
                m = re.search(r"commit(\d+)$", c["Strong"]["uri"])
                if m:
                    edges.append((e["index"], int(m.group(1))))

    weighted_edges = [(a, b, cosine(embeddings[a], embeddings[b])) for a, b in edges]

    node_ids = list(texts.keys())
    edge_set = set((a, b) for a, b, _ in weighted_edges) | set((b, a) for a, b, _ in weighted_edges)
    baseline_sims = [
        cosine(embeddings[node_ids[i]], embeddings[node_ids[j]])
        for i in range(len(node_ids))
        for j in range(i + 1, len(node_ids))
        if (node_ids[i], node_ids[j]) not in edge_set
    ]

    adj = collections.defaultdict(set)
    for a, b, _ in weighted_edges:
        adj[a].add(b)
        adj[b].add(a)
    visited, components = set(), []
    for n in node_ids:
        if n in visited:
            continue
        stack, comp = [n], set()
        while stack:
            cur = stack.pop()
            if cur in comp:
                continue
            comp.add(cur)
            visited.add(cur)
            stack.extend(adj[cur] - comp)
        components.append(comp)
    largest_component = max(components, key=len) if components else set()
    structural_connectivity = len(largest_component) / len(node_ids) if node_ids else None

    mean_edge_weight = float(np.mean([w for _, _, w in weighted_edges])) if weighted_edges else None
    baseline_mean = float(np.mean(baseline_sims)) if baseline_sims else None
    validated = [e for e in weighted_edges if baseline_mean is not None and e[2] > baseline_mean]
    validated_fraction = len(validated) / len(weighted_edges) if weighted_edges else None

    result = {
        "total_nodes": len(node_ids),
        "total_edges": len(weighted_edges),
        "structural_connectivity": structural_connectivity,
        "edges": [{"citer": a, "cited": b, "weight": w} for a, b, w in weighted_edges],
        "mean_edge_weight": mean_edge_weight,
        "baseline_mean_nonedge_similarity": baseline_mean,
        "n_baseline_pairs": len(baseline_sims),
        "validated_fraction": validated_fraction,
    }
    json.dump(result, open(out_path, "w"), indent=2)
    print(json.dumps(result, indent=2))
    return result


if __name__ == "__main__":
    analyze(sys.argv[1], sys.argv[2])
