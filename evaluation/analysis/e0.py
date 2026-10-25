"""E0: the test nets: how many nets each collection holds (from the manifests, before any run).

Counts the lines of every manifest per source (``source`` of the record), and for the generated nets (``gen``) the nets per
generator class and size range together with the number whose class, as detected by petrivet, equals the class the
generator aimed at. The library reports the narrowest class of a net, so a net generated for a wider class (general,
asymmetric choice) is detected as a narrower one when it happens to be: ``n_same_class`` is less than ``n`` for those
classes and that is no finding. The testkit writes a net once even if it was drawn twice (identical files have one content
hash), so a size range of a class can hold fewer nets than were drawn.
A net that is listed in two manifests (identical files drawn into two collections) is counted in each manifest and
reported as a duplicate: ``e0.n_duplicate_instances`` is the number of instance ids with more than one source.

Keys: e0.<source>.n_manifest_lines, e0.gen.<class>.<bucket>.{n,n_same_class}, e0.gen.<class>.n, e0.n_duplicate_instances,
      e0.n_unique_instances
Table: e0_nets.csv (source, gen_class, bucket, n, n_same_class, places_median)
"""

from __future__ import annotations

import statistics
from collections import Counter, defaultdict

import polars as pl

from .common import Ctx, read_jsonl


def run(ctx: Ctx) -> None:
    recs = []
    for p in ctx.manifests:
        if p.is_file():
            recs += read_jsonl(p)[0]
    if not recs:
        ctx.skip("e0", "no manifest found")
        return
    by_source = Counter(str(r.get("source")) for r in recs)
    for src, n in sorted(by_source.items()):
        ctx.num("e0", src, "n_manifest_lines", n)
    sources_of: dict[str, set] = defaultdict(set)
    for r in recs:
        if r.get("instance_id"):
            sources_of[r["instance_id"]].add(str(r.get("source")))
    ctx.num("e0", "n_unique_instances", len(sources_of))
    ctx.num("e0", "n_duplicate_instances", sum(1 for v in sources_of.values() if len(v) > 1))
    rows = []
    gen = [r for r in recs if r.get("source") == "gen"]
    groups: dict[tuple, list[dict]] = defaultdict(list)
    for r in gen:
        groups[(str(r.get("generator_class")), str(r.get("size_bucket")))].append(r)
    for (cls, bucket), g in sorted(groups.items()):
        same = sum(1 for r in g if str(r.get("detected_class_petrivet") or "").lower() == cls.lower())
        places = [r["places"] for r in g if isinstance(r.get("places"), (int, float))]
        rows.append({"source": "gen", "gen_class": cls, "bucket": bucket, "n": len(g), "n_same_class": same, "places_median": statistics.median(places) if places else None})
        ctx.num("e0", "gen", cls, bucket, "n", len(g))
        ctx.num("e0", "gen", cls, bucket, "n_same_class", same)
    for cls in sorted({c for c, _ in groups}):
        ctx.num("e0", "gen", cls, "n", sum(len(g) for (c, _), g in groups.items() if c == cls))
    for src, n in sorted(by_source.items()):
        if src == "gen":
            continue
        places = [r["places"] for r in recs if r.get("source") == src and isinstance(r.get("places"), (int, float))]
        rows.append({"source": src, "gen_class": None, "bucket": None, "n": n, "n_same_class": None, "places_median": statistics.median(places) if places else None})
    ctx.csv("e0_nets", pl.DataFrame(rows, infer_schema_length=None))
