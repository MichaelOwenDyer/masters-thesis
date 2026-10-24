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
from .common import PARTIAL_SOLVERS, QUERY_LABEL, SOLVER_COLOR, Ctx, color_map, key, plt, save_svg, solver_label, source_label, wrap
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
        # only the questions that every tool has a method for: a tool without one cannot be compared on it
        allq = t.filter(pl.col("source") == src)
        # a tool that answers only some of the questions (ITS-Tools) does not decide which questions are compared
        core = [s for s in solvers if s not in PARTIAL_SOLVERS]
        common = [q for q in sorted(allq["query"].unique().to_list())
                  if all(allq.filter((pl.col("query") == q) & (pl.col("solver_id") == s) & (pl.col("verdict") != "NOT_SUPPORTED")).height > 0 for s in core)]
        ts = allq.filter(pl.col("solved") & pl.col("query").is_in(common))
        partial = [s for s in solvers if s in PARTIAL_SOLVERS and ts.filter(pl.col("solver_id") == s).height > 0]
        # too few answers per tool for the curves to say anything (the calibration nets: 8 questions, all in about 7 ms)
        if ts.height == 0 or max(ts.filter(pl.col("solver_id") == s).height for s in core) < 30:
            continue
        fig, ax = p.subplots(figsize=(7.2, 3.8))
        for s in solvers:
            times = sorted(ts.filter(pl.col("solver_id") == s)["wall_ms"].to_list())
            if times:
                ax.plot(range(1, len(times) + 1), times, label=solver_label(s), color=SOLVER_COLOR.get(s, cm[s]), lw=1.6)
        if cap is not None:
            ax.axhline(cap, color="#999999", ls="--", lw=0.8)
            ax.text(0, cap * 1.12, "30 s limit", fontsize=7, color="#555555", va="bottom")
        ax.set_yscale("log")
        ax.set_xlabel("number of questions answered (quickest first)")
        ax.set_ylabel("time per answer (ms, log scale)")
        ax.set_title(f"How many questions each tool answers, and how fast: {source_label(src)}", loc="left", fontsize=10.5, pad=30)
        qs = ", ".join(QUERY_LABEL.get(q, q) for q in common)
        note = " ".join(f"{solver_label(s)} has no method for some of these questions, so its line covers fewer." for s in partial)
        ax.text(0, 1.02, wrap(f"Each point is one answer, sorted by time. A line that reaches further right answers more; a lower line is faster. Questions: {qs}. {note}".strip(), 105),
                transform=ax.transAxes, fontsize=7.2, color="#444444", va="bottom")
        ax.legend(frameon=False, fontsize=8, loc="upper left", bbox_to_anchor=(1.02, 1.0))
        save_svg(fig, ctx.fig_path(f"e4_cactus_{key(src)}"))
