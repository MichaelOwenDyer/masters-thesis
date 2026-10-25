"""E9: start-up cost: the run of every tool on a trivial net (the `BASELINE` runs of campaign B), wall time and peak memory.

One run per tool (the question is the first one the tool supports among Q1, Q3, Q2, Q4, Q5); the wall time is dominated by
starting the process (for ITS-Tools: the Java virtual machine and its reductions). The memory is the value of campaign M
for the same cell (see ``use_memory_campaign``): the largest single process of the run.

Keys: e9.<solver>.{baseline_wall_ms, baseline_rss_kib, query}
Table: e9_startup.csv
"""

from __future__ import annotations

import polars as pl

from .common import Ctx, series_max


def run(ctx: Ctx) -> None:
    b = ctx.runs.filter((pl.col("instance_id") == "BASELINE") & (pl.col("campaign") == "B") & (pl.col("rep") == 0))
    if b.height == 0:
        ctx.skip("e9", "no BASELINE runs in campaign B")
        return
    rows = []
    for s in sorted(b["solver_id"].unique().to_list()):
        g = b.filter(pl.col("solver_id") == s)
        wall = series_max(g["wall_ms"])  # one run per solver; the maximum of one value is that value
        rss = series_max(g["peak_rss_kib"])
        q = g["query"].to_list()[0]
        rows.append({"solver_id": s, "query": q, "baseline_wall_ms": wall, "baseline_rss_kib": rss, "verdict": g["verdict"].to_list()[0]})
        ctx.num("e9", s, "baseline_wall_ms", wall)
        ctx.num("e9", s, "baseline_rss_kib", rss)
        ctx.num("e9", s, "query", q)
    ctx.csv("e9_startup", pl.DataFrame(rows, infer_schema_length=None))
