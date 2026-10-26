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
  * Frontier: largest ``size_param`` at which every question of the family is solved (definite, not contradicting truth)
    per family and solver (``max_size_solved``), the largest size at which at least one question is solved
    (``max_size_any_question``), and the largest attempted.
  * Memory by size: the peak resident memory of both modes per family and size (median over the questions that have a
    memory value), with ``n_*_memout`` (runs ended by the memory limit, whose value is the limit) and ``n_*_no_value``
    (runs that were stopped by the time limit and therefore report no memory); a ratio of the medians would hide both.
  * RSS: paired median ``peak_rss_kib`` general / dispatch per class, two-sided Wilcoxon, Holm across classes (exploratory).
  * By size: the median speedup (t_general / t_dispatch) per family and size parameter, which does not depend on how many
    sizes were tried, with the cells of a size split by what was stopped: ``n_general_stopped`` (the speedup of such a cell
    is only a lower bound), ``n_dispatch_stopped`` (only an upper bound), ``n_both_stopped`` (no information, excluded from
    the median). Per family: ``last_clean_size`` (largest size at which no run of the family was stopped) with the
    median speedup of its cells, and the smallest size at which plain exploration / the automatic choice was stopped.
Keys: e2.<class>.{n_pairs,n_dropped,n_censored,n_nonzero,wilcoxon_p,holm_p,reject_h2,hl_ms,median_ratio,ratio_q1,ratio_q3,
      sens_n_pairs,sens_n_nonzero,sens_wilcoxon_p,sens_holm_p,sens_reject_h2,sens_hl_ms,sens_median_ratio,sens_ratio_q1,sens_ratio_q3,rss_pairs,rss_ratio_median,rss_p,rss_holm_p}
      e2.family.<family>.{last_clean_size,last_clean_speedup,general_first_stop_size,dispatch_first_stop_size}
      e2.frontier.<family>.<solver>.{max_size_solved,max_size_attempted}
      e2.mcc_small.{n_instances,n_non_general_class,n_poly_definite,n_dispatch_differs,share_dispatch_differs}
Figures: e2_scaling.svg (time vs size per family), e2_paired_scatter.svg, e2_speedup_by_size.svg, e2_memory_by_size.svg.
CSV: e2_speedup_by_size.csv, e2_family_summary.csv, e2_memory_by_size.csv.
"""

from __future__ import annotations

import numpy as np
import polars as pl

from .common import CLASS_LABEL, CLASS_ORDER, FAMILY_LABEL, family_rank, SOLVER_COLOR, Ctx, color_map, plt, save_svg, solver_label
from .stats import holm, hodges_lehmann, quantiles, wilcoxon_greater, wilcoxon_two_sided

D, G, P = "pv-dispatch", "pv-general", "pv-poly"
CENSORED = ("TIMEOUT", "MEMOUT")
DROPPED = ("CRASH", "INPUT_REJECTED", "NOT_SUPPORTED", "RUNNER_ERROR", "SHARD_FAILED")
ALPHA = 0.05


def paired(ctx: Ctx) -> pl.DataFrame:
    r = ctx.runs_meta("B").filter((pl.col("source") == "fam") & pl.col("solver_id").is_in([D, G]))
    if r.height == 0:
        return r
    agg = r.group_by("cls", "family", "size", "places", "instance_id", "query", "tid", "solver_id").agg(
        pl.col("verdict").sort_by("rep").first().alias("verdict"),
        pl.col("wall_ms").median().alias("wall"), pl.col("limit_ms").max().alias("cap"),
        pl.col("peak_rss_kib").median().alias("rss"),
    )
    agg = agg.with_columns(pl.when(pl.col("verdict").is_in(list(CENSORED))).then(pl.col("cap")).otherwise(pl.col("wall")).alias("t"),
                           pl.col("verdict").is_in(list(CENSORED)).alias("cens"))
    d = agg.filter(pl.col("solver_id") == D).drop("solver_id")
    g = agg.filter(pl.col("solver_id") == G).drop("solver_id")
    keys = ["cls", "family", "size", "places", "instance_id", "query", "tid"]
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
    ctx.csv("e2_pairs", ok.select("cls", "family", "size", "places", "instance_id", "query", "tid", "verdict_d", "verdict_g", "t_d", "t_g", "cens", "rss_d", "rss_g").sort("cls", "family", "size", "query", "tid"))
    _by_size(ctx, ok, [c for c in CLASS_ORDER if c in set(ok["cls"].to_list())] + sorted(set(ok["cls"].to_list()) - set(CLASS_ORDER)))
    _memory_by_size(ctx, ok, [c for c in CLASS_ORDER if c in set(ok["cls"].to_list())] + sorted(set(ok["cls"].to_list()) - set(CLASS_ORDER)))
    _figures(ctx, ok)


def _by_size(ctx: Ctx, ok: pl.DataFrame, classes: list[str]) -> None:
    """Median speedup per family and size, and the per-family summary that does not depend on the size grid."""
    k = ok.filter(pl.col("size").is_not_null()).with_columns(
        pl.col("size").cast(pl.Int64),
        (pl.col("cens_g") & ~pl.col("cens_d")).alias("g_only"),
        (pl.col("cens_d") & ~pl.col("cens_g")).alias("d_only"),
        (pl.col("cens_d") & pl.col("cens_g")).alias("both"),
        (pl.col("t_g").clip(lower_bound=1e-3) / pl.col("t_d").clip(lower_bound=1e-3)).alias("speedup"),
    )
    order = {c: i for i, c in enumerate(classes)}
    fams = sorted(k.group_by("family", "cls").len().iter_rows(named=True), key=lambda r: (order.get(r["cls"], 99), family_rank(r["family"]), r["family"]))
    rows, summary = [], []
    for fr in fams:
        f, c = fr["family"], fr["cls"]
        kf = k.filter(pl.col("family") == f)
        clean = []
        for size in sorted(kf["size"].unique().to_list()):
            ks = kf.filter(pl.col("size") == size)
            informative = ks.filter(~pl.col("both"))
            med = float(np.median(informative["speedup"].to_numpy())) if informative.height else None
            rows.append({
                "cls": c, "family": f, "size": size, "n_pairs": ks.height,
                "n_general_stopped": int(ks["g_only"].sum()), "n_dispatch_stopped": int(ks["d_only"].sum()), "n_both_stopped": int(ks["both"].sum()),
                "median_speedup": med,
            })
            if not ks["cens_g"].any() and not ks["cens_d"].any():
                clean.append((size, med))
        last = max(clean, key=lambda x: x[0]) if clean else (None, None)
        ctx.num("e2", "family", f, "last_clean_size", last[0])
        ctx.num("e2", "family", f, "last_clean_speedup", last[1])
        stops = {}
        for name, col in (("general_first_stop_size", "cens_g"), ("dispatch_first_stop_size", "cens_d")):
            st = kf.filter(pl.col(col))
            stops[name] = int(st["size"].min()) if st.height else None
            ctx.num("e2", "family", f, name, stops[name])
        summary.append({"cls": c, "family": f, "last_clean_size": last[0], "last_clean_speedup": last[1], **stops})
    df = pl.DataFrame(rows, infer_schema_length=None)
    ctx.csv("e2_speedup_by_size", df)
    ctx.csv("e2_family_summary", pl.DataFrame(summary, infer_schema_length=None))
    if not fams:
        return
    p = plt()
    colors = {"circuit": "#0072B2", "sm": "#D55E00", "mg": "#009E73", "efc": "#CC79A7", "ac": "#E69F00", "general": "#56B4E9"}
    n = len(fams)
    cols = min(3, n)
    nrows = (n + cols - 1) // cols
    fig, axs = p.subplots(nrows, cols, figsize=(3.7 * cols, 2.5 * nrows), squeeze=False, constrained_layout=True, sharex=True, sharey=True)
    for ax, fr in zip(axs.ravel(), fams):
        c = colors.get(fr["cls"], "#000000")
        d = df.filter((pl.col("family") == fr["family"]) & pl.col("median_speedup").is_not_null()).sort("size")
        ax.axhline(1, color="#777777", lw=0.9)
        ax.plot(d["size"], d["median_speedup"], color=c, lw=1.2)
        for r in d.iter_rows(named=True):
            g, dd = r["n_general_stopped"] > 0, r["n_dispatch_stopped"] > 0
            m = "D" if g and dd else "^" if g else "v" if dd else "o"
            ax.plot([r["size"]], [r["median_speedup"]], marker=m, ms=5, color=c, mfc="white" if (g or dd) else c, ls="none")
        ax.set_xscale("log", base=2); ax.set_yscale("log")
        ax.set_title(f"{CLASS_LABEL.get(fr['cls'], fr['cls']).split(' (')[0].capitalize()}: {FAMILY_LABEL.get(fr['family'], fr['family'])}", fontsize=8, loc="left")
        ax.tick_params(labelsize=7)
    for ax in axs.ravel()[n:]:
        ax.axis("off")
    for ax in axs[-1]:
        ax.set_xlabel("size parameter n of the family (log scale)", fontsize=8)
    for ax in axs[:, 0]:
        ax.set_ylabel("speedup (log scale)", fontsize=8)
    save_svg(fig, ctx.fig_path("e2_speedup_by_size"))


def _memory_by_size(ctx: Ctx, ok: pl.DataFrame, classes: list[str]) -> None:
    """Peak memory of both modes per family and size (MiB). A MEMOUT run keeps the memory limit as its value, a run
    stopped by the time limit has no value; both are counted so that the medians can be read with them."""
    k = ok.filter(pl.col("size").is_not_null()).with_columns(pl.col("size").cast(pl.Int64))
    order = {c: i for i, c in enumerate(classes)}
    fams = sorted(k.group_by("family", "cls").len().iter_rows(named=True), key=lambda r: (order.get(r["cls"], 99), family_rank(r["family"]), r["family"]))
    rows = []
    for fr in fams:
        kf = k.filter(pl.col("family") == fr["family"])
        for size in sorted(kf["size"].unique().to_list()):
            ks = kf.filter(pl.col("size") == size)
            row = {"cls": fr["cls"], "family": fr["family"], "size": size, "n_pairs": ks.height}
            for tag, rss, verdict in (("d", "rss_d", "verdict_d"), ("g", "rss_g", "verdict_g")):
                v = ks.filter(pl.col(rss).is_not_null() & (pl.col(rss) > 0))[rss]
                row[f"n_{tag}_with_value"] = len(v)
                row[f"n_{tag}_no_value"] = ks.height - len(v)
                row[f"n_{tag}_memout"] = int((ks[verdict] == "MEMOUT").sum())
                row[f"median_rss_{tag}_mib"] = float(np.median(v.to_numpy())) / 1024 if len(v) else None
            rows.append(row)
    if not rows:
        return
    df = pl.DataFrame(rows, infer_schema_length=None)
    ctx.csv("e2_memory_by_size", df)
    p = plt()
    gib = ctx.cfg.get("run", {}).get("mem_limit_gib")
    limit = float(gib) * 1024 if gib else float(df.select(pl.max_horizontal(pl.col("median_rss_d_mib").max(), pl.col("median_rss_g_mib").max())).item())
    n = len(fams)
    cols = min(3, n)
    nrows = (n + cols - 1) // cols
    fig, axs = p.subplots(nrows, cols, figsize=(3.7 * cols, 2.6 * nrows), squeeze=False, constrained_layout=True, sharex=True, sharey=True)
    for ax, fr in zip(axs.ravel(), fams):
        d = df.filter(pl.col("family") == fr["family"]).sort("size")
        for tag, solver in (("d", "pv-dispatch"), ("g", "pv-general")):
            dd = d.filter(pl.col(f"median_rss_{tag}_mib").is_not_null())
            ax.plot(dd["size"], dd[f"median_rss_{tag}_mib"], marker="o", ms=3, color=SOLVER_COLOR[solver], label=solver_label(solver))
            mo = dd.filter(pl.col(f"n_{tag}_memout") > 0)
            ax.plot(mo["size"], mo[f"median_rss_{tag}_mib"], ls="none", marker="o", ms=6, mfc="none", mec="#000000")
        ax.axhline(limit, color="#999999", ls="--", lw=0.8)
        ax.set_xscale("log", base=2); ax.set_yscale("log")
        ax.set_title(f"{CLASS_LABEL.get(fr['cls'], fr['cls']).split(' (')[0].capitalize()}: {FAMILY_LABEL.get(fr['family'], fr['family'])}", fontsize=8, loc="left")
        ax.tick_params(labelsize=7)
    for ax in axs.ravel()[n:]:
        ax.axis("off")
    for ax in axs[-1]:
        ax.set_xlabel("size parameter n of the family (log scale)", fontsize=8)
    for ax in axs[:, 0]:
        ax.set_ylabel("peak memory (MiB, log scale)", fontsize=8)
    h, lab = axs.ravel()[0].get_legend_handles_labels()
    fig.legend(h, lab, loc="outside upper left", ncol=2, frameon=False, fontsize=8)
    save_svg(fig, ctx.fig_path("e2_memory_by_size"))


def _figures(ctx: Ctx, ok: pl.DataFrame) -> None:
    p = plt()
    classes = [c for c in CLASS_ORDER if c in set(ok["cls"].to_list())] + sorted(set(ok["cls"].to_list()) - set(CLASS_ORDER))
    cm = dict(zip(classes, ["#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9"] + ["#000000"] * 4))
    cap = ctx.cap()
    # paired scatter: one dot per (net, question); above the diagonal the automatic choice is faster
    td = np.asarray(ok["t_d"].to_numpy(), dtype=float)
    tg = np.asarray(ok["t_g"].to_numpy(), dtype=float)
    finite = np.concatenate([td[np.isfinite(td) & (td > 0)], tg[np.isfinite(tg) & (tg > 0)]])
    lo, hi = float(finite.min()) / 2, float(finite.max()) * 3
    fig, ax = p.subplots(figsize=(6.6, 4.4))
    ax.set_autoscale_on(False)
    for c in classes:
        k = ok.filter(pl.col("cls") == c)
        ax.scatter(k["t_d"], k["t_g"], s=14, color=cm[c], label=CLASS_LABEL.get(c, c), alpha=0.75, linewidths=0)
    ax.plot([lo, hi], [lo, hi], color="#777777", lw=0.9)
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi); ax.set_ylim(lo, hi)
    # every point must be inside the axes; the limits are recorded so that an export shows what was used
    (x0, x1), (y0, y1) = ax.get_xlim(), ax.get_ylim()
    assert x0 <= td.min() and td.max() <= x1 and y0 <= tg.min() and tg.max() <= y1, f"scatter limits cut off data: {(x0, x1, y0, y1)}"
    for name, v in (("xlim_lo", x0), ("xlim_hi", x1), ("ylim_lo", y0), ("ylim_hi", y1)):
        ctx.num("e2", "scatter", name, v)
    if cap:
        ax.axhline(cap, color="#999999", ls="--", lw=0.8)
        ax.axvline(cap, color="#999999", ls="--", lw=0.8)
        ax.text(0.01, cap * 1.2, "30 s limit: run stopped without an answer", transform=ax.get_yaxis_transform(), fontsize=7, color="#555555", va="bottom")
    ax.set_xlabel("time with automatic dispatch (ms, log scale)")
    ax.set_ylabel("time with brute-force exploration (ms, log scale)")
    # which side is which goes next to the legend, outside the plotted area
    ax.text(1.02, 0.30, "Above the diagonal:\nautomatic dispatch faster", transform=ax.transAxes, fontsize=8, va="top", color="#333333")
    ax.text(1.02, 0.16, "Below the diagonal:\nbrute-force exploration faster", transform=ax.transAxes, fontsize=8, va="top", color="#333333")
    ax.legend(frameon=False, fontsize=8, title="Kind of net", title_fontsize=8.5, loc="upper left", bbox_to_anchor=(1.02, 1.0))
    save_svg(fig, ctx.fig_path("e2_paired_scatter"))
    # time against net size, one panel per family
    fams = ok.filter(pl.col("size").is_not_null()).group_by("family", "cls").len()
    if fams.height:
        order = {c: i for i, c in enumerate(CLASS_ORDER)}
        rows_list = sorted(fams.iter_rows(named=True), key=lambda r: (order.get(r["cls"], 99), family_rank(r["family"]), r["family"]))
        n = len(rows_list)
        cols = min(3, n)
        rows_ = (n + cols - 1) // cols
        fig, axs = p.subplots(rows_, cols, figsize=(3.7 * cols, 2.7 * rows_), squeeze=False, constrained_layout=True, sharex=True)
        for ax, r in zip(axs.ravel(), rows_list):
            k = ok.filter((pl.col("family") == r["family"]) & pl.col("size").is_not_null())
            for col, solver in (("t_d", "pv-dispatch"), ("t_g", "pv-general")):
                m = k.group_by("size").agg(pl.col(col).median(), pl.col("places").median().alias("places")).sort("places", "size")
                ax.plot(m["places"], m[col], marker="o", ms=3, label=solver_label(solver), color=SOLVER_COLOR[solver])
            if cap:
                ax.axhline(cap, color="#999999", ls="--", lw=0.8)
            ax.set_xscale("log"); ax.set_yscale("log")
            ax.set_title(f"{CLASS_LABEL.get(r['cls'], r['cls']).split(' (')[0].capitalize()}: {FAMILY_LABEL.get(r['family'], r['family'])}", fontsize=8, loc="left")
            ax.tick_params(labelsize=7)
        for ax in axs.ravel()[n:]:
            ax.axis("off")
        for ax in axs[-1]:
            ax.set_xlabel("number of places in the net (log scale)", fontsize=8)
        for ax in axs[:, 0]:
            ax.set_ylabel("time (ms, log scale)", fontsize=8)
        h, lab = axs.ravel()[0].get_legend_handles_labels()
        fig.legend(h, lab, loc="outside upper left", ncol=2, frameon=False, fontsize=8)
        save_svg(fig, ctx.fig_path("e2_scaling"))


def _frontier(ctx: Ctx) -> None:
    t = ctx.runs_meta("B").filter((pl.col("source") == "fam") & pl.col("solver_id").is_in([D, G]) & pl.col("size").is_not_null())
    if t.height == 0:
        return
    adj = ctx.adj.filter(pl.col("campaign") == "B").select("instance_id", "query", "tid", "solver_id", "outcome")
    t = t.filter(pl.col("rep") == 0).join(adj, on=["instance_id", "query", "tid", "solver_id"], how="left")
    t = t.with_columns((pl.col("verdict").is_in(["TRUE", "FALSE", "COUNT"]) & pl.col("outcome").ne("DISAGREE").fill_null(True)).alias("solved"))
    # a size counts as solved when every question of the family is answered at that size: a mode that answers a single
    # question (plain exploration decides boundedness of an unbounded net in milliseconds) has not handled the net
    per_size = t.group_by("family", "cls", "solver_id", "size").agg(
        pl.col("solved").all().alias("all_solved"), pl.col("solved").any().alias("any_solved"))
    f = per_size.group_by("family", "cls", "solver_id").agg(
        pl.col("size").filter(pl.col("all_solved")).max().alias("max_size_solved"),
        pl.col("size").filter(pl.col("any_solved")).max().alias("max_size_any_question"),
        pl.col("size").max().alias("max_size_attempted"),
        pl.col("size").n_unique().alias("n_sizes"),
    ).with_columns(pl.col("family").map_elements(family_rank, return_dtype=pl.Int64).alias("_rank")).sort("_rank", "family", "solver_id").drop("_rank")
    ctx.csv("e2_frontier", f)
    for r in f.iter_rows(named=True):
        ctx.num("e2", "frontier", r["family"], r["solver_id"], "max_size_solved", r["max_size_solved"])
        ctx.num("e2", "frontier", r["family"], r["solver_id"], "max_size_attempted", r["max_size_attempted"])
        ctx.num("e2", "frontier", r["family"], r["solver_id"], "max_size_any_question", r["max_size_any_question"])


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
