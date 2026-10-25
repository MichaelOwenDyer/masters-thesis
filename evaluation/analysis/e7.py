"""E7: memory. Peak RSS distribution and bytes per state.

Campaign B cells with the memory of campaign M (``use_memory_campaign`` in common.py: B's own peak memory contains the
runner's footprint), median over repetitions per cell. Distribution (p25 p50 p75 p95 max) per solver and source.
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

from .common import SOLVER_COLOR, SOLVER_SHORT, Ctx, color_map, plt, save_svg, series_max, solver_label, source_label, wrap
from .stats import quantiles


def run(ctx: Ctx) -> None:
    r = ctx.runs_meta("B").filter(pl.col("peak_rss_kib").is_not_null())
    if r.height == 0:
        ctx.skip("e7", "no campaign B runs with peak_rss_kib")
        return
    cells = r.group_by("source", "instance_id", "query", "tid", "solver_id").agg(pl.col("peak_rss_kib").median().alias("rss"), pl.col("states").max().alias("states"), pl.col("places").first().alias("places"),
                                                                              pl.col("verdict").sort_by("rep").first().alias("verdict")).sort("source", "solver_id", "instance_id", "query", "tid")
    rows = []
    for (src, s), g in cells.group_by("source", "solver_id"):
        q = quantiles(g["rss"].to_list(), (0.25, 0.5, 0.75, 0.95))
        row = {"source": src, "solver_id": s, "n": g.height, "rss_p25_kib": q[0.25], "rss_p50_kib": q[0.5], "rss_p75_kib": q[0.75], "rss_p95_kib": q[0.95], "rss_max_kib": series_max(g["rss"])}
        rows.append(row)
        for k, v in row.items():
            if k not in ("source", "solver_id"):
                ctx.num("e7", src, s, k, v)
    tab = pl.DataFrame(rows).sort("source", "solver_id")
    ctx.csv("e7_rss", tab)

    p = plt()
    from matplotlib.ticker import FuncFormatter
    order = ["fam", "mcc-small", "mcc-large"]  # calibration: every tool sits at the measuring floor, nothing to show
    srcs = [x for x in order if x in set(cells["source"].to_list())]
    solvers = [x for x in SOLVER_COLOR if x in set(cells["solver_id"].to_list())] + sorted(set(cells["solver_id"].to_list()) - set(SOLVER_COLOR))
    limit_mib = ctx.cfg.get("run", {}).get("mem_limit_gib", 8) * 1024
    ncols = 2
    nrows = (len(srcs) + ncols - 1) // ncols
    fig, axs = p.subplots(nrows, ncols, figsize=(8.6, 3.5 * nrows), squeeze=False, sharey=True)
    fig.subplots_adjust(top=0.80, bottom=0.08, left=0.08, right=0.98, hspace=0.42, wspace=0.12)
    for ax, src in zip(axs.ravel(), srcs):
        data, labs = [], []
        for s in solvers:
            v = cells.filter((pl.col("source") == src) & (pl.col("solver_id") == s))["rss"].to_numpy() / 1024.0
            if len(v):
                data.append(v); labs.append(s)
        if data:
            bp = ax.boxplot(data, patch_artist=True, showfliers=True, flierprops={"markersize": 2.5, "markeredgewidth": 0.4, "alpha": 0.6})
            for patch, s in zip(bp["boxes"], labs):
                patch.set_facecolor(SOLVER_COLOR.get(s, "#999999")); patch.set_alpha(0.6)
            for med in bp["medians"]:
                med.set_color("#000000")
            ax.set_xticks(range(1, len(labs) + 1)); ax.set_xticklabels([f"{SOLVER_SHORT.get(s, s)}\n(n = {len(v)})" for s, v in zip(labs, data)], fontsize=7)
        ax.set_yscale("log")
        ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _: f"{v:g}"))
        ax.axhline(limit_mib, color="#999999", ls="--", lw=0.8)
        ax.text(0.99, limit_mib * 0.85, "8 GiB limit", transform=ax.get_yaxis_transform(), fontsize=6.5, color="#555555", va="top", ha="right")
        ax.set_title(source_label(src), fontsize=9, loc="left")
        ax.set_ylabel("peak memory (MiB, log scale)", fontsize=8)
        ax.tick_params(labelsize=7)
    for ax in axs.ravel()[len(srcs):]:
        ax.axis("off")
    fig.text(0.01, 0.955, "How much memory does each tool use?", fontsize=10.5, ha="left")
    fig.text(0.01, 0.925, "One run per question and net (n = number of runs that gave an answer). Runs at the dashed 8 GiB line ran out of memory.\n"
             "About 7 MiB is the size of the measuring wrapper itself: those runs used almost nothing.",
             fontsize=7, color="#444444", ha="left", va="top")
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
    plotted = 0
    fig, ax = p.subplots(figsize=(7.0, 3.8))
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
        places = g["places"].to_numpy()
        keep = (v > 1) & np.isfinite(places)  # at or below the start-up memory there is nothing to measure
        plotted += int(keep.sum())
        ax.scatter(places[keep], v[keep], s=10, label=solver_label(s), color=SOLVER_COLOR.get(s, "#999999"), alpha=0.8, linewidths=0)
    if out:
        ctx.csv("e7_bytes_per_state", pl.DataFrame(out))
        ax.set_xscale("log"); ax.set_yscale("log")
        ax.set_xlabel("number of places in the net (log scale)"); ax.set_ylabel("memory per state (bytes, log scale)")
        ax.set_title("How much memory does a tool need per state it explores?", loc="left", fontsize=10.5, pad=30)
        ax.text(0, 1.02, wrap("Counting all states of a net. Memory above the tool's start-up memory, divided by the number of states; nets too small to measure are not drawn. Lower is better.", 105),
                transform=ax.transAxes, fontsize=7.2, color="#444444", va="bottom")
        ax.legend(frameon=False, fontsize=8, loc="upper left", bbox_to_anchor=(1.02, 1.0))
        if plotted:
            save_svg(fig, ctx.fig_path("e7_bytes_per_state"))
        else:
            p.close(fig)
            ctx.skip("e7_bytes_per_state_figure", "no net with a known number of places (is the manifest given?)")
