"""E7: memory. Peak RSS distribution and bytes per state.

Campaign B, median ``peak_rss_kib`` over repetitions per cell. Distribution (p25 p50 p75 p95 max) per solver and source.
Bytes per state = (peak_rss_kib - baseline) * 1024 / states, on Q0 runs with verdict COUNT and states > 0; baseline = median
``peak_rss_kib`` of the solver's ``instance_id == "BASELINE"`` runs (no-op on a trivial net); without a baseline for that
solver the solver is skipped for bytes per state. Only solvers that report ``states`` (COUNT) appear.
Keys: e7.<source>.<solver>.{n,rss_p25_kib,rss_p50_kib,rss_p75_kib,rss_p95_kib,rss_max_kib}
      e7.bytes_per_state.<solver>.{n,median,p90,baseline_kib}
Figures: e7_rss.svg (box plot per solver, per source panel), e7_bytes_per_state.svg (scatter over states).
"""

from __future__ import annotations

import numpy as np
import polars as pl

from .common import Ctx, color_map, plt, save_svg
from .stats import quantiles


def run(ctx: Ctx) -> None:
    r = ctx.runs_meta("B").filter(pl.col("peak_rss_kib").is_not_null())
    if r.height == 0:
        ctx.skip("e7", "no campaign B runs with peak_rss_kib")
        return
    cells = r.group_by("source", "instance_id", "query", "tid", "solver_id").agg(pl.col("peak_rss_kib").median().alias("rss"), pl.col("states").max().alias("states"),
                                                                              pl.col("verdict").sort_by("rep").first().alias("verdict")).sort("source", "solver_id", "instance_id", "query", "tid")
    rows = []
    for (src, s), g in cells.group_by("source", "solver_id"):
        q = quantiles(g["rss"].to_list(), (0.25, 0.5, 0.75, 0.95))
        row = {"source": src, "solver_id": s, "n": g.height, "rss_p25_kib": q[0.25], "rss_p50_kib": q[0.5], "rss_p75_kib": q[0.75], "rss_p95_kib": q[0.95], "rss_max_kib": g["rss"].max()}
        rows.append(row)
        for k, v in row.items():
            if k not in ("source", "solver_id"):
                ctx.num("e7", src, s, k, v)
    tab = pl.DataFrame(rows).sort("source", "solver_id")
    ctx.csv("e7_rss", tab)

    p = plt()
    srcs = sorted(cells["source"].unique().to_list())
    solvers = sorted(cells["solver_id"].unique().to_list())
    cm = color_map(solvers)
    fig, axs = p.subplots(1, len(srcs), figsize=(3.4 * len(srcs), 3.2), squeeze=False)
    for ax, src in zip(axs[0], srcs):
        data, labs = [], []
        for s in solvers:
            v = cells.filter((pl.col("source") == src) & (pl.col("solver_id") == s))["rss"].to_numpy() / 1024.0
            if len(v):
                data.append(v); labs.append(s)
        if data:
            bp = ax.boxplot(data, patch_artist=True, showfliers=True)
            for patch, s in zip(bp["boxes"], labs):
                patch.set_facecolor(cm[s]); patch.set_alpha(0.6)
            ax.set_xticks(range(1, len(labs) + 1)); ax.set_xticklabels(labs, rotation=30, ha="right", fontsize=7)
        ax.set_yscale("log"); ax.set_title(src, fontsize=9); ax.set_ylabel("peak RSS (MiB)")
    save_svg(fig, ctx.fig_path("e7_rss"))

    # bytes per state
    base = ctx.runs.filter((pl.col("instance_id") == "BASELINE") & pl.col("peak_rss_kib").is_not_null()).group_by("solver_id").agg(pl.col("peak_rss_kib").median().alias("base"))
    base_map = dict(zip(base["solver_id"].to_list(), base["base"].to_list()))
    q0 = cells.filter((pl.col("query") == "Q0") & (pl.col("verdict") == "COUNT") & (pl.col("states") > 0))
    if q0.height == 0:
        ctx.skip("e7_bytes_per_state", "no Q0 COUNT runs with states")
        return
    if not base_map:
        ctx.skip("e7_bytes_per_state", "no BASELINE runs with peak_rss_kib")
        return
    out = []
    fig, ax = p.subplots(figsize=(4.8, 3.4))
    for s in sorted(q0["solver_id"].unique().to_list()):
        if s not in base_map:
            ctx.numbers["e7.bytes_per_state.%s.note" % s.replace("-", "_")] = "no baseline run for this solver"
            continue
        g = q0.filter(pl.col("solver_id") == s).with_columns(((pl.col("rss") - base_map[s]) * 1024.0 / pl.col("states")).alias("bps"))
        v = g["bps"].to_numpy()
        q = quantiles(v, (0.5, 0.9))
        out.append({"solver_id": s, "n": g.height, "baseline_kib": base_map[s], "median_bytes_per_state": q[0.5], "p90_bytes_per_state": q[0.9]})
        for k, val in (("n", g.height), ("median", q[0.5]), ("p90", q[0.9]), ("baseline_kib", base_map[s])):
            ctx.num("e7", "bytes_per_state", s, k, val)
        ax.scatter(g["states"], np.clip(v, 1, None), s=10, label=s, color=cm.get(s), alpha=0.8, linewidths=0)
    if out:
        ctx.csv("e7_bytes_per_state", pl.DataFrame(out))
        ax.set_xscale("log"); ax.set_yscale("log")
        ax.set_xlabel("states (Q0)"); ax.set_ylabel("bytes per state")
        ax.legend(frameon=False, fontsize=7)
        save_svg(fig, ctx.fig_path("e7_bytes_per_state"))
