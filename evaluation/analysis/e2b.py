"""E2b: the automatic choice (`pv-dispatch`) against plain exploration (`pv-general`) on the competition models, before and
after the library fix of the Karp-Miller acceleration (SPEC amendment "the automatic choice is slower than plain
exploration on the benchmark models").

"Before": the `pv-dispatch` records of campaign B archived in ``results/archive/pv-dispatch-before-omega-fix.jsonl.gz``
(an optional input: without it only "after" is computed); "after": the records of the current ``runs.jsonl``. The
`pv-general` records are the same in both. Per source (`mcc-small`, `mcc-large`) and question (Q1, Q3, Q4, Q5):
  n_dispatch_answered / n_general_answered   cells with a definite answer (TRUE or FALSE)
  n_both                                     cells that both answered
  ratio_median, ratio_p90                    ratio of the times dispatch / general over those cells (> 1: dispatch slower);
                                             time of a cell = median wall time over its repetitions;
                                             p90 = the sorted value at index int(0.9 (n - 1))
  n_dispatch_fails_general_answers           dispatch timed out or ran out of memory where general answered
Keys: e2b.<state>.<source>.<query>.<field>, state is `before` or `after`, and `e2b.before_present`
Table: e2b_dispatch_mcc.csv
"""

from __future__ import annotations

import gzip
import json
import statistics
from collections import defaultdict

import polars as pl

from .common import RUN_SCHEMA, Ctx, frame, with_meta

ARCHIVE = "archive/pv-dispatch-before-omega-fix.jsonl.gz"
QUERIES = ("Q1", "Q3", "Q4", "Q5")
DEFINITE = ("TRUE", "FALSE")


def _cells(df: pl.DataFrame, solver: str) -> dict[tuple, tuple[str, float]]:
    """(instance, query) -> (verdict of repetition 0, median wall time over the repetitions) of one solver in campaign B."""
    out: dict[tuple, tuple[str, list[float], str]] = {}
    d = df.filter((pl.col("solver_id") == solver) & (pl.col("instance_id") != "BASELINE") & pl.col("query").is_in(list(QUERIES)))
    walls: dict[tuple, list[float]] = defaultdict(list)
    verdict: dict[tuple, str] = {}
    source: dict[tuple, str] = {}
    for r in d.iter_rows(named=True):
        k = (r["instance_id"], r["query"])
        walls[k].append(r["wall_ms"])
        if r["rep"] == 0:
            verdict[k] = r["verdict"]
        source[k] = r["source"]
    return {k: (verdict[k], statistics.median(walls[k]), source[k]) for k in verdict}  # type: ignore[misc]


def _row(state: str, source: str, query: str, disp: dict, gen: dict) -> dict:
    keys = [k for k in disp if k[1] == query and disp[k][2] == source]
    d_ans = [k for k in keys if disp[k][0] in DEFINITE]
    g_ans = [k for k in gen if k[1] == query and gen[k][2] == source and gen[k][0] in DEFINITE]
    both = [k for k in d_ans if k in gen and gen[k][0] in DEFINITE]
    ratios = sorted(disp[k][1] / gen[k][1] for k in both if gen[k][1] > 0)
    fails = [k for k in keys if disp[k][0] in ("TIMEOUT", "MEMOUT") and k in gen and gen[k][0] in DEFINITE]
    return {
        "state": state, "source": source, "query": query, "n_cells": len(keys), "n_dispatch_answered": len(d_ans), "n_general_answered": len(g_ans),
        "n_both": len(both), "ratio_median": statistics.median(ratios) if ratios else None,
        "ratio_p90": ratios[int(0.9 * (len(ratios) - 1))] if ratios else None, "n_dispatch_fails_general_answers": len(fails),
    }


def run(ctx: Ctx) -> None:
    cur = ctx.runs_meta("B")
    gen = _cells(cur.filter(pl.col("source").is_in(["mcc-small", "mcc-large"])), "pv-general")
    after = _cells(cur.filter(pl.col("source").is_in(["mcc-small", "mcc-large"])), "pv-dispatch")
    if not gen or not after:
        ctx.skip("e2b", "no pv-dispatch / pv-general runs on the competition models in campaign B")
        return
    states = [("after", after)]
    path = ctx.results / ARCHIVE
    ctx.numbers["e2b.before_present"] = path.is_file()
    if path.is_file():
        with gzip.open(path, "rt", encoding="utf-8") as f:
            rows = [json.loads(line) for line in f if line.strip()]
        old = frame(rows, RUN_SCHEMA).with_columns(pl.col("target_id").fill_null("").alias("tid")).filter(pl.col("campaign") == "B")
        states.insert(0, ("before", _cells(with_meta(old, ctx.meta), "pv-dispatch")))
    table = []
    for state, disp in states:
        for source in ("mcc-small", "mcc-large"):
            for q in QUERIES:
                row = _row(state, source, q, disp, gen)
                if row["n_cells"] == 0:
                    continue
                table.append(row)
                for field, v in row.items():
                    if field not in ("state", "source", "query"):
                        ctx.num("e2b", state, source, q, field, v)
    ctx.csv("e2b_dispatch_mcc", pl.DataFrame(table, infer_schema_length=None))
