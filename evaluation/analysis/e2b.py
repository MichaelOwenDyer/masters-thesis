"""E2b: the automatic choice (`pv-dispatch`) against plain exploration (`pv-general`) on the competition models, before and
after the library optimization of the Karp-Miller acceleration (SPEC amendment "the automatic choice is slower than plain
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
  Every cell falls into exactly one of six categories (n_cells = their sum), none is left out:
  n_faster2 / n_equal2 / n_slower2           both answered, and the speedup general / dispatch is >= 2, strictly between 0.5 and 2, or <= 0.5
  n_only_general                             general answered, dispatch gave no definite answer
  n_only_dispatch                            dispatch answered, general gave no definite answer
  n_neither                                  neither answered
Keys: e2b.<state>.<source>.<query>.<field>, state is `before` or `after`, and `e2b.before_present`
Table: e2b_dispatch_mcc.csv
Figure: e2b_scatter.svg (small competition models: time of dispatch against time of general, one dot per model and question,
        one panel per question and state; a run without a definite answer is drawn at the wall cap)
"""

from __future__ import annotations

import gzip
import json
import statistics
from collections import defaultdict

import polars as pl

from .common import QUERY_LABEL, RUN_SCHEMA, Ctx, frame, plt, save_svg, with_meta

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
    speedups = [gen[k][1] / disp[k][1] if disp[k][1] > 0 else float("inf") for k in both]
    only_general = [k for k in keys if k not in d_ans and k in gen and gen[k][0] in DEFINITE]
    only_dispatch = [k for k in d_ans if k not in gen or gen[k][0] not in DEFINITE]
    return {
        "state": state, "source": source, "query": query, "n_cells": len(keys), "n_dispatch_answered": len(d_ans), "n_general_answered": len(g_ans),
        "n_both": len(both), "ratio_median": statistics.median(ratios) if ratios else None,
        "ratio_p90": ratios[int(0.9 * (len(ratios) - 1))] if ratios else None, "n_dispatch_fails_general_answers": len(fails),
        "n_faster2": sum(1 for x in speedups if x >= 2.0), "n_slower2": sum(1 for x in speedups if x <= 0.5),
        "n_equal2": sum(1 for x in speedups if 0.5 < x < 2.0), "n_only_general": len(only_general), "n_only_dispatch": len(only_dispatch),
        "n_neither": len(keys) - len(both) - len(only_general) - len(only_dispatch),
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
    _scatter(ctx, gen, states)


OUTCOME = (("both", "both answered", "#0072B2"), ("only_general", "only plain exploration answered", "#D55E00"),
           ("only_dispatch", "only the automatic choice answered", "#009E73"), ("neither", "neither answered", "#000000"))


def _scatter(ctx: Ctx, gen: dict, states: list) -> None:
    """One dot per (small competition model, question): time of the automatic choice (x) against plain exploration (y).
    A run without a definite answer is drawn at the wall cap, on the dashed line, so the failures are visible.
    The legend lists only the outcomes that occur in the data."""
    cap = ctx.cap() or 30000.0
    pts: dict[tuple[str, str], list[tuple[float, float, str]]] = {}
    for state, disp in states:
        for q in QUERIES:
            for k, (verdict, wall, source) in disp.items():
                if k[1] != q or source != "mcc-small" or k not in gen:
                    continue
                gv, gw, _ = gen[k]
                d_ok, g_ok = verdict in DEFINITE, gv in DEFINITE
                outcome = "both" if d_ok and g_ok else "only_general" if g_ok else "only_dispatch" if d_ok else "neither"
                pts.setdefault((state, q), []).append((wall if d_ok else cap, gw if g_ok else cap, outcome))
    if not pts:
        return
    times = [t for v in pts.values() for x, y, _ in v for t in (x, y) if t > 0]
    lo, hi = min(times) / 2, cap * 3
    p = plt()
    names = [st for st, _ in states]
    used: set[str] = set()
    fig, axs = p.subplots(len(names), len(QUERIES), figsize=(3.3 * len(QUERIES), 3.2 * len(names)), squeeze=False, sharex=True, sharey=True, constrained_layout=True)
    for r, st in enumerate(names):
        for c, q in enumerate(QUERIES):
            ax = axs[r][c]
            ax.set_autoscale_on(False)
            for key_, _, color in OUTCOME:
                v = [(x, y) for x, y, o in pts.get((st, q), []) if o == key_]
                if v:
                    used.add(key_)
                    if key_ == "neither":  # a few models, drawn large and on top so that they are not lost in the corner
                        ax.scatter([x for x, _ in v], [y for _, y in v], s=34, color=color, linewidths=1.2, marker="x", zorder=5)
                    else:
                        ax.scatter([x for x, _ in v], [y for _, y in v], s=14, color=color, alpha=0.6, linewidths=0)
            ax.plot([lo, hi], [lo, hi], color="#777777", lw=0.9)
            ax.axhline(cap, color="#999999", ls="--", lw=0.8)
            ax.axvline(cap, color="#999999", ls="--", lw=0.8)
            ax.set_xscale("log"); ax.set_yscale("log"); ax.set_xlim(lo, hi); ax.set_ylim(lo, hi)
            ax.set_title(f"{QUERY_LABEL[q]}, {'before' if st == 'before' else 'after'} optimization" if len(names) > 1 else QUERY_LABEL[q], fontsize=8.5, loc="left")
            ax.tick_params(labelsize=7)
    for ax in axs[-1]:
        ax.set_xlabel("time with automatic dispatch (ms, log scale)", fontsize=8)
    for ax in axs[:, 0]:
        ax.set_ylabel("time with reachability graph\nexploration (ms, log scale)", fontsize=8)
    from matplotlib.lines import Line2D
    fig.legend(handles=[Line2D([], [], marker="x" if k == "neither" else "o", color=col, ls="none", label=label) for k, label, col in OUTCOME if k in used],  # only the outcomes that occur
               loc="outside lower center", ncol=4, frameon=False, fontsize=8)
    save_svg(fig, ctx.fig_path("e2b_scatter"))
