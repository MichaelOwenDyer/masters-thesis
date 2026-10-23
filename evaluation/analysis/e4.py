"""E4: comparison with the reference tools, campaign B, per source and query.

Cell = (instance, query, target). The cell set of a (source, query) is the union over all solvers; a solver without a
record in a cell (and every NOT_SUPPORTED, TIMEOUT, MEMOUT, UNKNOWN, error) counts as unsolved. solved = definite verdict
that does not disagree with truth (wrong answers are counted separately as n_wrong). Time = median wall_ms over the
repetitions. PAR-2 = mean over the cells of (time if solved within the cap, else 2 x cap), one common cap for all
solvers (eval.toml [run].wall_cap_secs, else the largest limit_ms of campaign B).
Coverage columns: n_cells, n_supported (not NOT_SUPPORTED), n_definite, n_unknown, n_timeout (TIMEOUT+MEMOUT), n_error
(CRASH+INPUT_REJECTED+RUNNER_ERROR+SHARD_FAILED), n_missing (no record).

Keys: e4.<source>.<query>.<solver>.{n_cells,n_solved,n_wrong,par2_ms,n_supported,n_definite,n_unknown,n_timeout,n_error,n_missing,median_solved_ms}
      e4.<source>.<solver>.<same fields> (queries pooled within a source, sources are never pooled)
Figure: e4_cactus_<source>.svg (x = number of solved cells, y = time, cells sorted per solver).
"""

from __future__ import annotations

import polars as pl

from .cells import cell_table
from .common import Ctx, color_map, plt, save_svg, key
from .stats import median


def summarise(t: pl.DataFrame, cell_keys: list[str], n_cells: int, cap: float | None) -> dict:
    n = t.height
    solved = t.filter(pl.col("solved"))
    par2 = None
    if cap is not None and n_cells:
        par2 = (solved.filter(pl.col("wall_ms") <= cap)["wall_ms"].sum() + 2 * cap * (n_cells - solved.filter(pl.col("wall_ms") <= cap).height)) / n_cells
    v = t["verdict"]
    return {
        "n_cells": n_cells, "n_missing": n_cells - n, "n_solved": solved.height,
        "n_wrong": t.filter(pl.col("outcome") == "DISAGREE").height,
        "par2_ms": par2, "median_solved_ms": median(solved["wall_ms"].to_list()),
        "n_supported": int((v != "NOT_SUPPORTED").sum()), "n_definite": int(v.is_in(["TRUE", "FALSE", "COUNT"]).sum()),
        "n_unknown": int((v == "UNKNOWN").sum()), "n_timeout": int(v.is_in(["TIMEOUT", "MEMOUT"]).sum()),
        "n_error": int(v.is_in(["CRASH", "INPUT_REJECTED", "RUNNER_ERROR", "SHARD_FAILED"]).sum()),
    }


def run(ctx: Ctx) -> None:
    t = cell_table(ctx, "B")
    if t.height == 0:
        ctx.skip("e4", "no campaign B runs")
        return
    cap = ctx.cap()
    if cap is None:
        ctx.skip("e4_par2", "no cap available (eval.toml [run].wall_cap_secs missing and no limit_ms)")
    solvers = sorted(t["solver_id"].unique().to_list())
    rows = []
    for src in sorted(t["source"].unique().to_list()):
        ts = t.filter(pl.col("source") == src)
        for q in [None] + sorted(ts["query"].unique().to_list()):
            tq = ts if q is None else ts.filter(pl.col("query") == q)
            n_cells = tq.select("instance_id", "query", "tid").unique().height
            for s in solvers:
                d = summarise(tq.filter(pl.col("solver_id") == s), [], n_cells, cap)
                rows.append({"source": src, "query": q or "all", "solver_id": s, **d})
                for f, val in d.items():
                    ctx.num("e4", src, *([q] if q else []), s, f, val)
    ctx.csv("e4", pl.DataFrame(rows))

    # cactus plots, one per source
    cm = color_map(solvers)
    p = plt()
    for src in sorted(t["source"].unique().to_list()):
        ts = t.filter((pl.col("source") == src) & pl.col("solved"))
        if ts.height == 0:
            continue
        fig, ax = p.subplots(figsize=(5.2, 3.6))
        for s in solvers:
            times = sorted(ts.filter(pl.col("solver_id") == s)["wall_ms"].to_list())
            if times:
                ax.plot(range(1, len(times) + 1), times, label=s, color=cm[s], lw=1.4)
        if cap is not None:
            ax.axhline(cap, color="#999999", ls="--", lw=0.8)
        ax.set_yscale("log")
        ax.set_xlabel("instances solved")
        ax.set_ylabel("wall time per instance (ms)")
        ax.set_title(f"Cactus plot, {src}")
        ax.legend(frameon=False)
        save_svg(fig, ctx.fig_path(f"e4_cactus_{key(src)}"))
