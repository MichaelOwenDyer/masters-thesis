"""E6: latency. p50 / p95 / p99 per variant, startup baseline separate.

Input ``results/latency.jsonl`` (path override ``--latency``), schema of petrivet-eval/src/latency.rs (``LatencyRecord``):
campaign "D", kind ("call" | "baseline"), variant (pv-inproc-parsed, pv-inproc-cached, pv-inproc-parse, pv-dispatch, lola, tina,
baseline-noop, baseline-pv-version), query?, instance_id?, class?, position (index in the sequence of calls), phase
("cold" = position 0 | "steady"), ns (latency in nanoseconds), ms, verdict, process_first (first in-process call of the whole
run: library initialisation, Z3 load), aborted, ...
Used here: ``variant``, ``kind``, ``phase``, ``ns`` (else ``latency_us`` / ``latency_ns`` / ``latency_ms`` / ``ms``),
``process_first``, ``verdict``. Records whose ``verdict`` is present and not TRUE/FALSE/COUNT are dropped (no answer, so no
latency of an answer; counted as ``n_dropped_no_answer``). Phase of a row: ``baseline`` if kind is "baseline", ``process_first``
if that flag is set (reported separately, not under cold), else the record's ``phase`` (default steady).
Fallback when latency.jsonl is absent: subprocess runs of campaign D in runs.jsonl (``wall_ms``) become variants
``subprocess:<solver_id>``; their ``instance_id == "BASELINE"`` runs are the baseline.
Per (variant, phase): n, p50, p95, p99 (numpy linear quantiles), min, max, in microseconds.
Keys: e6.<variant>.<phase>.{n,p50_us,p95_us,p99_us,min_us,max_us}; phase in cold steady baseline process_first;
      e6.n_dropped_no_answer.
Figure: e6_latency.svg (p50 / p95 / p99 per variant, steady state, log scale).
"""

from __future__ import annotations

import polars as pl

from .common import Ctx, color_map, plt, read_jsonl, save_svg
from .stats import quantiles

UNIT = (("ns", 1e-3), ("latency_us", 1.0), ("latency_ns", 1e-3), ("latency_ms", 1e3), ("ms", 1e3))
OK = ("TRUE", "FALSE", "COUNT")


def load_latency(path) -> tuple[pl.DataFrame, int]:
    rows, _ = read_jsonl(path)
    out, dropped = [], 0
    for r in rows:
        v = r.get("variant")
        us = next((float(r[k]) * f for k, f in UNIT if r.get(k) is not None), None)
        if v is None or us is None:
            continue
        if r.get("verdict") is not None and r["verdict"] not in OK:
            dropped += 1
            continue
        if r.get("kind") == "baseline":
            phase = "baseline"
        elif r.get("process_first"):
            phase = "process_first"
        else:
            phase = str(r.get("phase") or "steady")
        out.append({"variant": str(v), "phase": phase, "us": us})
    return pl.DataFrame(out, schema={"variant": pl.Utf8, "phase": pl.Utf8, "us": pl.Float64}), dropped


def from_runs(ctx: Ctx) -> pl.DataFrame:
    d = ctx.runs.filter((pl.col("campaign") == "D") & pl.col("wall_ms").is_not_null())
    if d.height == 0:
        return pl.DataFrame(schema={"variant": pl.Utf8, "phase": pl.Utf8, "us": pl.Float64})
    return d.select(
        ("subprocess:" + pl.col("solver_id")).alias("variant"),
        pl.when(pl.col("instance_id") == "BASELINE").then(pl.lit("baseline")).otherwise(pl.lit("steady")).alias("phase"),
        (pl.col("wall_ms") * 1e3).alias("us"),
    )


def run(ctx: Ctx) -> None:
    p = ctx.latency_path
    if p is not None and p.is_file():
        df, dropped = load_latency(p)
        ctx.num("e6", "n_dropped_no_answer", dropped)
    else:
        if p is not None:
            ctx.numbers["e6.latency_file_note"] = f"{p} not found; using campaign D rows of runs.jsonl if any"
        df = from_runs(ctx)
    if df.height == 0:
        ctx.skip("e6", "no results/latency.jsonl and no campaign D runs")
        return
    rows = []
    for (variant, phase), g in df.group_by("variant", "phase"):
        q = quantiles(g["us"].to_list(), (0.5, 0.95, 0.99))
        row = {"variant": variant, "phase": phase, "n": g.height, "p50_us": q[0.5], "p95_us": q[0.95], "p99_us": q[0.99], "min_us": g["us"].min(), "max_us": g["us"].max()}
        rows.append(row)
        for k in ("n", "p50_us", "p95_us", "p99_us", "min_us", "max_us"):
            ctx.num("e6", variant, phase, k, row[k])
    tab = pl.DataFrame(rows).sort("variant", "phase")
    ctx.csv("e6", tab)
    st = tab.filter(pl.col("phase") == "steady")
    if st.height:
        cm = color_map(["p50", "p95", "p99"])
        fig, ax = plt().subplots(figsize=(5.6, 3.4))
        n = st.height
        w = 0.26
        for i, c in enumerate(["p50", "p95", "p99"]):
            ax.bar([x + (i - 1) * w for x in range(n)], st[f"{c}_us"].to_list(), width=w, label=c, color=cm[c])
        ax.set_xticks(range(n))
        ax.set_xticklabels(st["variant"].to_list(), rotation=30, ha="right")
        ax.set_yscale("log")
        ax.set_ylabel("latency (us)")
        ax.legend(frameon=False)
        save_svg(fig, ctx.fig_path("e6_latency"))
