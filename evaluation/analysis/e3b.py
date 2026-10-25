"""E3b: cost control by timeout. Campaign C records of ``pv-dispatch`` with ``timeout_ms`` set.

overshoot = analyze_ms - timeout_ms (parse time excluded). Per (instance, query, target, t) the median over repetitions is
one observation (a "cell"). A cell is *triggered* when the library deadline actually fired: verdict UNKNOWN, or
overshoot > 0; only triggered cells enter the distribution statistics (``overshoot_all`` columns include the rest).
Threshold for H3b ("close to its deadline"): ``--overshoot-threshold-ms`` or eval.toml ``[e3].overshoot_threshold_ms``
(or ``[e3b].overshoot_threshold_ms``); without one the share-within-threshold is skipped (distribution only).
Group = manifest ``family`` (mcc_family for MCC), else source. The procedure/phase responsible for the largest overshoots is
not in the run record (the CLI reports no method); the largest cells are listed in e3b_top_overshoot.csv for the manual
perf follow-up of the pilot.
Runs that did not return at all (the wall cap of the campaign ended them, so they report no analysis time) are not part of the
distribution: they are counted per t in ``e3b.t<t>.n_wall_cap`` and listed in e3b_wall_cap.csv (the library did not stop at its deadline).
Keys: e3b.t<t>.n_wall_cap, e3b.t<t>.{n_cells,n_triggered,share_unknown,share_definite,overshoot_median_ms,overshoot_p90_ms,overshoot_p95_ms,
      overshoot_max_ms,share_within_threshold,wall_minus_analyze_median_ms}; e3b.<source>.t<t>.<same>; e3b.threshold_ms
Figure: e3b_overshoot.svg (histogram of triggered overshoot per t, log-spaced bins on max(overshoot, 0.01 ms)).
"""

from __future__ import annotations

import numpy as np
import polars as pl

from .common import Ctx, plt, save_svg, color_map
from .stats import quantiles

DEFINITE = ["TRUE", "FALSE", "COUNT"]


def _threshold(ctx: Ctx) -> float | None:
    if ctx.overshoot_threshold_ms is not None:
        return ctx.overshoot_threshold_ms
    for sec in ("e3", "e3b"):
        v = ctx.cfg.get(sec, {}).get("overshoot_threshold_ms")
        if v is not None:
            return float(v)
    return None


def _row(g: pl.DataFrame, thr: float | None) -> dict:
    trig = g.filter(pl.col("triggered"))
    o = trig["overshoot_ms"].to_list()
    q = quantiles(o, (0.5, 0.9, 0.95))
    n = g.height
    out = {
        "n_cells": n, "n_triggered": trig.height,
        "share_unknown": g.filter(pl.col("verdict") == "UNKNOWN").height / n if n else None,
        "share_definite": g.filter(pl.col("verdict").is_in(DEFINITE)).height / n if n else None,
        "overshoot_median_ms": q[0.5], "overshoot_p90_ms": q[0.9], "overshoot_p95_ms": q[0.95],
        "overshoot_max_ms": max(o) if o else None,
        "wall_minus_analyze_median_ms": float(np.median(g["wall_minus_analyze_ms"].drop_nulls().to_numpy())) if g["wall_minus_analyze_ms"].drop_nulls().len() else None,
    }
    if thr is not None:
        out["share_within_threshold"] = (trig.filter(pl.col("overshoot_ms") <= thr).height / trig.height) if trig.height else None
    return out


def wall_cap_cells(c: pl.DataFrame) -> pl.DataFrame:
    """The cells (instance, query, target, t) of campaign C whose run was ended by the wall cap: TIMEOUT, one row per cell."""
    return (c.filter(pl.col("verdict") == "TIMEOUT")
            .group_by("source", "family", "instance_id", "query", "tid", "timeout_ms").agg(pl.col("wall_ms").median())
            .with_columns(pl.col("family").fill_null(pl.col("source")).alias("group"))
            .select("source", "group", "instance_id", "query", "tid", "timeout_ms", "wall_ms")
            .sort("source", "instance_id", "query", "tid", "timeout_ms"))


def run(ctx: Ctx) -> None:
    c_all = ctx.runs_meta("C").filter((pl.col("solver_id") == ctx.subject) & pl.col("timeout_ms").is_not_null())
    if c_all.height:
        wc = wall_cap_cells(c_all)
        ctx.csv("e3b_wall_cap", wc)
        for t in sorted(c_all["timeout_ms"].unique().to_list()):
            ctx.num("e3b", f"t{int(t)}", "n_wall_cap", wc.filter(pl.col("timeout_ms") == t).height)
    r = ctx.runs_meta("C").filter((pl.col("solver_id") == ctx.subject) & pl.col("timeout_ms").is_not_null() & pl.col("analyze_ms").is_not_null())
    if r.height == 0:
        ctx.skip("e3b", "no campaign C runs with timeout_ms and analyze_ms")
        return
    r = r.with_columns((pl.col("analyze_ms") - pl.col("timeout_ms")).alias("overshoot_ms"),
                       (pl.col("wall_ms") - pl.col("analyze_ms") - pl.col("parse_ms").fill_null(0)).alias("wall_minus_analyze_ms"))
    cells = r.group_by("source", "family", "instance_id", "query", "tid", "timeout_ms").agg(
        pl.col("overshoot_ms").median(), pl.col("verdict").sort_by("rep").first(), pl.col("wall_ms").median(),
        pl.col("wall_minus_analyze_ms").median(), pl.col("parse_ms").median(), pl.col("analyze_ms").median(),
    ).sort("source", "instance_id", "query", "tid", "timeout_ms").with_columns(((pl.col("verdict") == "UNKNOWN") | (pl.col("overshoot_ms") > 0)).alias("triggered"))
    cells = cells.with_columns(pl.col("family").fill_null(pl.col("source")).alias("group"))
    thr = _threshold(ctx)
    if thr is None:
        ctx.numbers["e3b.threshold_note"] = "no overshoot threshold configured: distribution only (H3b threshold is set by the pilot)"
    else:
        ctx.num("e3b", "threshold_ms", thr)
    rows = []
    ts = sorted(cells["timeout_ms"].unique().to_list())
    for t in ts:
        ct = cells.filter(pl.col("timeout_ms") == t)
        d = _row(ct, thr)
        rows.append({"scope": "all", "timeout_ms": t, **d})
        for k, v in d.items():
            ctx.num("e3b", f"t{int(t)}", k, v)
        for src in sorted(ct["source"].unique().to_list()):
            ds = _row(ct.filter(pl.col("source") == src), thr)
            rows.append({"scope": src, "timeout_ms": t, **ds})
            for k, v in ds.items():
                ctx.num("e3b", src, f"t{int(t)}", k, v)
        for grp in sorted(ct["group"].unique().to_list()):
            rows.append({"scope": f"family:{grp}", "timeout_ms": t, **_row(ct.filter(pl.col("group") == grp), thr)})
    ctx.csv("e3b_overshoot", pl.DataFrame(rows, infer_schema_length=None))
    top = cells.filter(pl.col("triggered")).sort("overshoot_ms", descending=True).head(20)
    ctx.csv("e3b_top_overshoot", top.select("source", "group", "instance_id", "query", "tid", "timeout_ms", "overshoot_ms", "verdict", "parse_ms", "analyze_ms", "wall_ms"))

    p = plt()
    trig = cells.filter(pl.col("triggered"))
    if trig.height:
        cm = dict(zip([str(int(t)) for t in ts], ["#56B4E9", "#E69F00", "#009E73", "#0072B2"]))
        fig, ax = p.subplots(figsize=(6.4, 3.8))
        vals = np.clip(trig["overshoot_ms"].to_numpy(), 0.01, None)
        bins = np.logspace(np.log10(vals.min()) - 0.05, np.log10(vals.max()) + 0.05, 25) if vals.max() > vals.min() else 10
        for t in ts:
            v = np.clip(trig.filter(pl.col("timeout_ms") == t)["overshoot_ms"].to_numpy(), 0.01, None)
            if len(v):
                ax.hist(v, bins=bins, alpha=0.65, label=f"{int(t)} ms", color=cm[str(int(t))])
        ax.set_xscale("log")
        if thr is not None:
            ax.axvline(thr, color="#000000", ls="--", lw=0.9)
            ax.text(thr * 1.15, ax.get_ylim()[1] * 0.97, f"{thr:g} ms: our limit for\n\"close to the deadline\"", fontsize=7.5, va="top")
        ax.set_xlabel("how long after the time limit the library stopped (ms, log scale)")
        ax.set_ylabel("number of queries")
        ax.legend(frameon=False, title="time limit given", title_fontsize=8.5, fontsize=8, loc="upper left", bbox_to_anchor=(1.02, 1.0))
        save_svg(fig, ctx.fig_path("e3b_overshoot"))
