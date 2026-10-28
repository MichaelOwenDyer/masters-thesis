"""E4: comparison with the reference tools, campaign B, per source and query.

Cell = (instance, query, target). The cell set of a (source, query) is the union over all solvers; a solver without a
record in a cell (and every NOT_SUPPORTED, TIMEOUT, MEMOUT, UNKNOWN, error) counts as unsolved. solved = definite verdict
that does not disagree with truth (wrong answers are counted separately as n_wrong). Time = median wall_ms over the
repetitions. PAR-2 = mean over the cells of: the time, if the cell is solved within the cap or the tool declined it
(verdict UNKNOWN before the cap: an unknown answer is not held against the tool); else 2 x cap (time-out, memory-out, error,
no record, an unknown at the cap). One common cap for all solvers (eval.toml [run].wall_cap_secs, else the largest
limit_ms of campaign B).
Coverage columns: n_cells, n_supported (not NOT_SUPPORTED), n_definite, n_unknown, n_timeout (TIMEOUT+MEMOUT), n_error
(CRASH+INPUT_REJECTED+RUNNER_ERROR+SHARD_FAILED), n_missing (no record).

Keys: e4.<source>.<query>.<solver>.{n_cells,n_solved,n_wrong,par2_ms,n_supported,n_definite,n_unknown,n_timeout,n_error,n_missing,median_solved_ms}
      e4.<source>.<solver>.<same fields> (queries pooled within a source, sources are never pooled)
Figure: e4_cactus_<source>.svg (one panel per question: x = number of answered cells, y = time, quickest first, one line per tool).
"""

from __future__ import annotations

import polars as pl

from .cells import cell_table
from .common import QUERY_LABEL, SOLVER_COLOR, Ctx, color_map, key, plt, save_svg, solver_label
from .stats import median


def summarise(t: pl.DataFrame, cell_keys: list[str], n_cells: int, cap: float | None) -> dict:
    n = t.height
    solved = t.filter(pl.col("solved"))
    par2 = None
    if cap is not None and n_cells:
        answered = solved.filter(pl.col("wall_ms") <= cap)
        declined = t.filter((pl.col("verdict") == "UNKNOWN") & (pl.col("wall_ms") < cap))  # an unknown before the deadline costs its time only
        par2 = (answered["wall_ms"].sum() + declined["wall_ms"].sum() + 2 * cap * (n_cells - answered.height - declined.height)) / n_cells
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

    # cactus plots: one figure per source, one panel per question (a tool appears where it answers)
    p = plt()
    cm = color_map(solvers)
    cap_ms = cap
    for src in sorted(t["source"].unique().to_list()):
        ts = t.filter((pl.col("source") == src) & pl.col("solved"))
        qs = [q for q in sorted(ts["query"].unique().to_list()) if ts.filter(pl.col("query") == q).height > 0]
        # too few answers per tool for the curves to say anything (the calibration nets: 8 questions, all in about 7 ms)
        if not qs or max(ts.filter(pl.col("solver_id") == s).height for s in solvers) < 30:
            continue
        cols = min(3, len(qs))
        nrows = (len(qs) + cols - 1) // cols
        fig, axs = p.subplots(nrows, cols, figsize=(3.6 * cols, 2.9 * nrows + 0.5), squeeze=False, sharey=True, constrained_layout=True)
        for ax, q in zip(axs.ravel(), qs):
            for s in solvers:
                times = sorted(ts.filter((pl.col("solver_id") == s) & (pl.col("query") == q))["wall_ms"].to_list())
                if times:
                    ax.plot(range(1, len(times) + 1), times, label=solver_label(s), color=SOLVER_COLOR.get(s, cm[s]), lw=1.5)
            if cap_ms is not None:
                ax.axhline(cap_ms, color="#999999", ls="--", lw=0.8)
            n_cells = t.filter((pl.col("source") == src) & (pl.col("query") == q)).select("instance_id", "query", "tid").unique().height
            ax.set_xlim(0, n_cells)
            ax.set_yscale("log")
            ax.set_title(QUERY_LABEL.get(q, q), fontsize=8.5, loc="left")
            ax.tick_params(labelsize=7)
        for ax in axs.ravel()[len(qs):]:
            ax.axis("off")
        for ax in axs[-1]:
            ax.set_xlabel("number of questions answered (quickest first)", fontsize=8)
        for ax in axs[:, 0]:
            ax.set_ylabel("time per answer (ms, log scale)", fontsize=8)
        h, lab = axs.ravel()[0].get_legend_handles_labels()
        seen = {}
        for ax in axs.ravel():
            for hh, ll in zip(*ax.get_legend_handles_labels()):
                seen.setdefault(ll, hh)
        fig.legend(list(seen.values()), list(seen.keys()), loc="outside upper center", ncol=3, frameon=False, fontsize=8)
        save_svg(fig, ctx.fig_path(f"e4_cactus_{key(src)}"))
