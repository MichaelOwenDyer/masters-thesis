"""E2: dispatch ablation, `fam` only (SPEC 11 E2), plus the share of `mcc-small` where dispatch differs from general.

Pairs: one pair per (instance, query, target) of source ``fam`` (campaign B) that has a record of both ``pv-dispatch`` and
``pv-general``. Time = median ``wall_ms`` over the repetitions; a TIMEOUT/MEMOUT run is censored at its cap (``limit_ms``).
Pairs where either solver CRASHed, rejected the input, was NOT_SUPPORTED or had a RUNNER_ERROR are dropped (counted as
``n_dropped``). Pairs of all queries of a class are pooled (the class is the unit of H2).
Difference d = t_general - t_dispatch (positive: dispatch faster).
  * Wilcoxon signed-rank, one-sided H1: median(d) > 0, zero differences dropped (``scipy.stats.wilcoxon``,
    zero_method="wilcox"), alpha 0.05, Holm-corrected across the classes (separately for primary and sensitivity).
  * Hodges-Lehmann estimate of d (ms) and median ratio t_general / t_dispatch with its quartiles (IQR).
  * Censoring: the primary analysis keeps censored pairs at the cap (both censored: d = 0, dropped by the test);
    ``n_censored`` counts pairs with at least one censored side. The sensitivity run (``sens``) excludes them.
  * Frontier: largest ``size_param`` solved (definite, not contradicting truth) per family and solver, and the largest
    attempted.
  * RSS: paired median ``peak_rss_kib`` general / dispatch per class, two-sided Wilcoxon, Holm across classes (exploratory).
Keys: e2.<class>.{n_pairs,n_dropped,n_censored,n_nonzero,wilcoxon_p,holm_p,reject_h2,hl_ms,median_ratio,ratio_q1,ratio_q3,
      sens_n_pairs,sens_n_nonzero,sens_wilcoxon_p,sens_holm_p,sens_reject_h2,sens_hl_ms,sens_median_ratio,sens_ratio_q1,sens_ratio_q3,rss_pairs,rss_ratio_median,rss_p,rss_holm_p}
      e2.frontier.<family>.<solver>.{max_size_solved,max_size_attempted}
      e2.mcc_small.{n_instances,n_non_general_class,n_poly_definite,n_dispatch_differs,share_dispatch_differs}
Figures: e2_scaling.svg (time vs size per family), e2_paired_scatter.svg.
"""

from __future__ import annotations

import numpy as np
import polars as pl

from .common import Ctx, color_map, plt, save_svg
from .stats import holm, hodges_lehmann, quantiles, wilcoxon_greater, wilcoxon_two_sided

D, G, P = "pv-dispatch", "pv-general", "pv-poly"
CENSORED = ("TIMEOUT", "MEMOUT")
DROPPED = ("CRASH", "INPUT_REJECTED", "NOT_SUPPORTED", "RUNNER_ERROR", "SHARD_FAILED")
ALPHA = 0.05


def paired(ctx: Ctx) -> pl.DataFrame:
    r = ctx.runs_meta("B").filter((pl.col("source") == "fam") & pl.col("solver_id").is_in([D, G]))
    if r.height == 0:
        return r
    agg = r.group_by("cls", "family", "size", "instance_id", "query", "tid", "solver_id").agg(
        pl.col("verdict").sort_by("rep").first().alias("verdict"),
        pl.col("wall_ms").median().alias("wall"), pl.col("limit_ms").max().alias("cap"),
        pl.col("peak_rss_kib").median().alias("rss"),
    )
    agg = agg.with_columns(pl.when(pl.col("verdict").is_in(list(CENSORED))).then(pl.col("cap")).otherwise(pl.col("wall")).alias("t"),
                           pl.col("verdict").is_in(list(CENSORED)).alias("cens"))
    d = agg.filter(pl.col("solver_id") == D).drop("solver_id")
    g = agg.filter(pl.col("solver_id") == G).drop("solver_id")
    keys = ["cls", "family", "size", "instance_id", "query", "tid"]
    return d.join(g, on=keys, suffix="_g").rename({"verdict": "verdict_d", "t": "t_d", "cens": "cens_d", "wall": "wall_d", "rss": "rss_d", "cap": "cap_d"}).rename(
        {"verdict_g": "verdict_g", "t_g": "t_g", "cens_g": "cens_g"})


def _stats(d: np.ndarray, tg: np.ndarray, td: np.ndarray) -> dict:
    ratio = np.clip(tg, 1e-3, None) / np.clip(td, 1e-3, None)
    q = quantiles(ratio, (0.25, 0.5, 0.75))
    _, p = wilcoxon_greater(d)
    return {"n_pairs": len(d), "n_nonzero": int((d != 0).sum()), "wilcoxon_p": p, "hl_ms": hodges_lehmann(d),
            "median_ratio": q[0.5], "ratio_q1": q[0.25], "ratio_q3": q[0.75]}


def run(ctx: Ctx) -> None:
    pairs = paired(ctx)
    if pairs.height == 0:
        ctx.skip("e2", "no pv-dispatch/pv-general pairs on source fam in campaign B (is the manifest given?)")
    else:
        _ablation(ctx, pairs)
        _frontier(ctx)
    _mcc_small(ctx)


def _ablation(ctx: Ctx, pairs: pl.DataFrame) -> None:
    n_all = pairs.group_by("cls").len().rename({"len": "n_all"})
    ok = pairs.filter(~pl.col("verdict_d").is_in(list(DROPPED)) & ~pl.col("verdict_g").is_in(list(DROPPED)))
    ok = ok.with_columns((pl.col("cens_d") | pl.col("cens_g")).alias("cens")).sort("cls", "family", "size", "instance_id", "query", "tid")
    classes = sorted(ok["cls"].unique().to_list())
    rows = []
    for c in classes:
        k = ok.filter(pl.col("cls") == c)
        d = (k["t_g"] - k["t_d"]).to_numpy()
        prim = _stats(d, k["t_g"].to_numpy(), k["t_d"].to_numpy())
        ks = k.filter(~pl.col("cens"))
        sens = _stats((ks["t_g"] - ks["t_d"]).to_numpy(), ks["t_g"].to_numpy(), ks["t_d"].to_numpy()) if ks.height else {"n_pairs": 0, "n_nonzero": 0, "wilcoxon_p": None, "hl_ms": None, "median_ratio": None, "ratio_q1": None, "ratio_q3": None}
        n_total = int(n_all.filter(pl.col("cls") == c)["n_all"][0])
        # paired RSS
        kr = k.filter(pl.col("rss_d").is_not_null() & pl.col("rss_g").is_not_null() & (pl.col("rss_d") > 0))
        rss_ratio = (kr["rss_g"] / kr["rss_d"]).to_numpy()
        _, rss_p = wilcoxon_two_sided((kr["rss_g"] - kr["rss_d"]).to_numpy()) if kr.height else (None, None)
        rows.append({"cls": c, "n_dropped": n_total - k.height, "n_censored": int(k["cens"].sum()), "prim": prim, "sens": sens,
                     "rss_pairs": kr.height, "rss_ratio_median": float(np.median(rss_ratio)) if len(rss_ratio) else None, "rss_p": rss_p})
    hp = holm([r["prim"]["wilcoxon_p"] for r in rows])
    hs = holm([r["sens"]["wilcoxon_p"] for r in rows])
    hr = holm([r["rss_p"] for r in rows])
    flat = []
    for r, p, s, rr in zip(rows, hp, hs, hr):
        r["prim"]["holm_p"], r["sens"]["holm_p"], r["rss_holm_p"] = p, s, rr
        c = r["cls"]
        out = {"cls": c, "n_dropped": r["n_dropped"], "n_censored": r["n_censored"]}
        out.update(r["prim"])
        out["reject_h2"] = p is not None and p < ALPHA
        out.update({f"sens_{k}": v for k, v in r["sens"].items()})
        out["sens_reject_h2"] = s is not None and s < ALPHA
        out.update({"rss_pairs": r["rss_pairs"], "rss_ratio_median": r["rss_ratio_median"], "rss_p": r["rss_p"], "rss_holm_p": rr})
        flat.append(out)
        for k, v in out.items():
            if k == "cls":
                continue
            ctx.num("e2", c, k, v)
    ctx.csv("e2_paired_tests", pl.DataFrame(flat, infer_schema_length=None))
    ctx.num("e2", "n_classes_tested", len(flat))
    ctx.num("e2", "n_classes_h2_rejected", sum(1 for f in flat if f["reject_h2"]))
    ctx.num("e2", "alpha", ALPHA)
    ctx.csv("e2_pairs", ok.select("cls", "family", "size", "instance_id", "query", "tid", "verdict_d", "verdict_g", "t_d", "t_g", "cens", "rss_d", "rss_g").sort("cls", "family", "size", "query", "tid"))
    _figures(ctx, ok)


def _figures(ctx: Ctx, ok: pl.DataFrame) -> None:
    p = plt()
    # paired scatter
    cm = color_map(ok["cls"].unique().to_list())
    fig, ax = p.subplots(figsize=(4.4, 4.2))
    for c in sorted(cm):
        k = ok.filter(pl.col("cls") == c)
        ax.scatter(k["t_d"], k["t_g"], s=12, color=cm[c], label=c, alpha=0.8, linewidths=0)
    lo, hi = max(ok["t_d"].min(), 1e-1) / 2, max(ok["t_d"].max(), ok["t_g"].max()) * 2
    ax.plot([lo, hi], [lo, hi], color="#999999", lw=0.8)
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("pv-dispatch wall time (ms)"); ax.set_ylabel("pv-general wall time (ms)")
    ax.set_title("Paired times, fam (censored at the cap)")
    ax.legend(frameon=False, fontsize=7)
    save_svg(fig, ctx.fig_path("e2_paired_scatter"))
    # scaling per family
    fams = ok.filter(pl.col("size").is_not_null()).group_by("family", "cls").len().sort("cls", "family")
    if fams.height:
        n = fams.height
        cols = min(3, n)
        rows_ = (n + cols - 1) // cols
        fig, axs = p.subplots(rows_, cols, figsize=(3.6 * cols, 2.9 * rows_), squeeze=False, constrained_layout=True)
        for ax, r in zip(axs.ravel(), fams.iter_rows(named=True)):
            k = ok.filter((pl.col("family") == r["family"]) & pl.col("size").is_not_null())
            for col, lab, colr in (("t_d", "pv-dispatch", "#0072B2"), ("t_g", "pv-general", "#D55E00")):
                m = k.group_by("size").agg(pl.col(col).median()).sort("size")
                ax.plot(m["size"], m[col], marker="o", ms=3, label=lab, color=colr)
            ax.set_xscale("log", base=2); ax.set_yscale("log")
            ax.set_title(f"{r['family']} ({r['cls']})", fontsize=8)
            ax.set_xlabel("size parameter"); ax.set_ylabel("time (ms)")
        for ax in axs.ravel()[n:]:
            ax.axis("off")
        axs.ravel()[0].legend(frameon=False, fontsize=7)
        save_svg(fig, ctx.fig_path("e2_scaling"))


def _frontier(ctx: Ctx) -> None:
    t = ctx.runs_meta("B").filter((pl.col("source") == "fam") & pl.col("solver_id").is_in([D, G]) & pl.col("size").is_not_null())
    if t.height == 0:
        return
    adj = ctx.adj.filter(pl.col("campaign") == "B").select("instance_id", "query", "tid", "solver_id", "outcome")
    t = t.filter(pl.col("rep") == 0).join(adj, on=["instance_id", "query", "tid", "solver_id"], how="left")
    t = t.with_columns((pl.col("verdict").is_in(["TRUE", "FALSE", "COUNT"]) & pl.col("outcome").ne("DISAGREE").fill_null(True)).alias("solved"))
    f = t.group_by("family", "cls", "solver_id").agg(
        pl.col("size").filter(pl.col("solved")).max().alias("max_size_solved"),
        pl.col("size").max().alias("max_size_attempted"),
        pl.col("size").n_unique().alias("n_sizes"),
    ).sort("family", "solver_id")
    ctx.csv("e2_frontier", f)
    for r in f.iter_rows(named=True):
        ctx.num("e2", "frontier", r["family"], r["solver_id"], "max_size_solved", r["max_size_solved"])
        ctx.num("e2", "frontier", r["family"], r["solver_id"], "max_size_attempted", r["max_size_attempted"])


def _mcc_small(ctx: Ctx) -> None:
    """Share of mcc-small instances on which dispatch can differ from general: detected class is not `general`
    (structural procedures apply), or pv-poly gives a definite verdict on at least one query."""
    r = ctx.runs_meta(("A", "B")).filter(pl.col("source") == "mcc-small")
    if r.height == 0:
        ctx.skip("e2_mcc_small", "no mcc-small runs")
        return
    inst = r.select("instance_id", "cls").unique()
    nongen = inst.filter(~pl.col("cls").is_in(["general", "unknown"]))["instance_id"].to_list()
    poly = r.filter((pl.col("solver_id") == P) & pl.col("verdict").is_in(["TRUE", "FALSE", "COUNT"]))["instance_id"].unique().to_list()
    either = set(nongen) | set(poly)
    n = inst.height
    ctx.num("e2", "mcc_small", "n_instances", n)
    ctx.num("e2", "mcc_small", "n_non_general_class", len(nongen))
    ctx.num("e2", "mcc_small", "n_poly_definite", len(poly))
    ctx.num("e2", "mcc_small", "n_dispatch_differs", len(either))
    ctx.num("e2", "mcc_small", "share_dispatch_differs", len(either) / n if n else None)
    if (inst["cls"] == "unknown").all():
        ctx.numbers["e2.mcc_small.note"] = "no class information (manifest missing): n_non_general_class is 0 by default"
    ctx.csv("e2_mcc_small_classes", inst.group_by("cls").len().rename({"len": "n_instances"}).sort("cls"))
