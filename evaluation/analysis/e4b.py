"""E4b: petrivet against the reference tools, per model and per size (the views that a table of solved counts and a
median cannot give). Cells come from ``cells.cell_table`` (campaign B, memory of campaign M).

petrivet = ``pv-dispatch`` (the automatic dispatch); references = ``lola`` and ``tina``. A cell is *answered* when its verdict
is definite and does not contradict the truth (``solved``); a run without an answer is drawn at the wall cap.

Competition models (``mcc-small``, ``mcc-large``), questions Q1, Q3, Q4, Q5:
  e4b_time_headtohead.csv      per source, question and reference: n_cells and the categories (n_both with the speedup
                               reference time / petrivet time at least 2 = n_faster2, strictly between 0.5 and 2 = n_equal2,
                               at most 0.5 = n_slower2; n_only_petrivet, n_only_reference, n_neither; they sum to n_cells)
  e4b_memory_headtohead.csv    per source, question and reference: the cells in which both have a memory value (answered:
                               the memory pass; memory-out: the limit; time-out: the memory of campaign B when the run was
                               killed, a lower bound), n_points, n_with_timeout_value, n_with_memout, n_without_value, and
                               how many of the points petrivet used at least twice / at most half the memory of the reference
  figures e4b_time_scatter_<source>.svg, e4b_memory_scatter_<source>.svg (rows: reference, columns: question)
Scaling families (``fam``, questions Q1 to Q4; petrivet, LoLA, TINA):
  e4b_scaling_fam.csv          per family, size and tool: total_ms (sum over the questions, a question without an answer
                               counts as the cap), n_failed, max_rss_mib (largest peak memory of any question, with the same
                               rule), n_memout, n_timeout_value
  e4b_frontier.csv             per family and tool: the largest size at which every question is answered
  figures e4b_scaling_time_fam.svg, e4b_scaling_memory_fam.svg
"""

from __future__ import annotations

import math

import polars as pl

from .cells import cell_table
from .common import CLASS_ORDER, FAMILY_LABEL, QUERY_LABEL, SOLVER_COLOR, Ctx, family_rank, plt, save_svg, solver_label

PETRIVET = "pv-dispatch"
REFERENCES = ("lola", "tina")
MCC_QUESTIONS = ("Q1", "Q3", "Q4", "Q5")
FAM_TOOLS = (PETRIVET, "lola", "tina")
OUTCOMES = (("both", "both answered", "#0072B2"), ("only_petrivet", "only petrivet answered", "#009E73"),
            ("only_reference", "only the reference tool answered", "#D55E00"), ("neither", "neither answered", "#000000"))
TOOL_NAME = {"lola": "LoLA", "tina": "TINA"}


def _memory(row: dict) -> tuple[float | None, str]:
    """(MiB, status) of one cell: the memory pass for an answer, the limit for a memory-out, B's value for a time-out."""
    v = row["verdict"]
    if v == "TIMEOUT":
        m, status = row["rss_b_kib"], "time-out"
    elif v == "MEMOUT":
        m, status = row["rss_kib"] if row["rss_kib"] is not None else row["rss_b_kib"], "memory-out"
    else:
        m, status = row["rss_kib"], "answered"
    return (m / 1024 if m is not None else None), status


def run(ctx: Ctx) -> None:
    t = cell_table(ctx, "B")
    if t.height == 0:
        ctx.skip("e4b", "no campaign B runs")
        return
    cap = ctx.cap() or 30000.0
    cells: dict[tuple, dict[str, dict]] = {}
    for r in t.iter_rows(named=True):
        cells.setdefault((r["source"], r["instance_id"], r["query"], r["tid"]), {})[r["solver_id"]] = r
    _mcc(ctx, cells, cap)
    _fam(ctx, t, cap)


# ---------------------------------------------------------------------------------------------- competition models

def _mcc(ctx: Ctx, cells: dict, cap: float) -> None:
    time_rows, mem_rows = [], []
    time_pts: dict[tuple, list] = {}
    mem_pts: dict[tuple, list] = {}
    for (src, _inst, q, _tid), by in cells.items():
        if src not in ("mcc-small", "mcc-large") or q not in MCC_QUESTIONS or PETRIVET not in by:
            continue
        for ref in REFERENCES:
            if ref not in by:
                continue
            a, b = by[PETRIVET], by[ref]
            ta, tb = (a["wall_ms"] if a["solved"] else cap), (b["wall_ms"] if b["solved"] else cap)
            outcome = "both" if a["solved"] and b["solved"] else "only_petrivet" if a["solved"] else "only_reference" if b["solved"] else "neither"
            time_pts.setdefault((src, ref, q), []).append((ta, tb, outcome))
            (ma, sa), (mb, sb) = _memory(a), _memory(b)
            mem_pts.setdefault((src, ref, q), []).append((ma, mb, sa, sb))
    if not time_pts:
        ctx.skip("e4b_mcc", "no competition-model cells for petrivet and a reference tool")
        return
    for (src, ref, q), pts in sorted(time_pts.items()):
        both = [(ta, tb) for ta, tb, o in pts if o == "both"]
        speed = [tb / ta if ta > 0 else math.inf for ta, tb in both]
        time_rows.append({
            "source": src, "query": q, "reference": ref, "n_cells": len(pts), "n_both": len(both),
            "n_faster2": sum(1 for x in speed if x >= 2.0), "n_equal2": sum(1 for x in speed if 0.5 < x < 2.0), "n_slower2": sum(1 for x in speed if x <= 0.5),
            "n_only_petrivet": sum(1 for *_, o in pts if o == "only_petrivet"), "n_only_reference": sum(1 for *_, o in pts if o == "only_reference"),
            "n_neither": sum(1 for *_, o in pts if o == "neither"),
        })
        m = mem_pts[(src, ref, q)]
        have = [(a, b, sa, sb) for a, b, sa, sb in m if a is not None and b is not None]
        ratio = [b / a for a, b, *_ in have if a > 0]
        mem_rows.append({
            "source": src, "query": q, "reference": ref, "n_cells": len(m), "n_points": len(have),
            "n_with_timeout_value": sum(1 for *_, sa, sb in have if "time-out" in (sa, sb)), "n_with_memout": sum(1 for *_, sa, sb in have if "memory-out" in (sa, sb)),
            "n_without_value": len(m) - len(have),
            "n_petrivet_less2": sum(1 for x in ratio if x >= 2.0), "n_similar2": sum(1 for x in ratio if 0.5 < x < 2.0), "n_petrivet_more2": sum(1 for x in ratio if x <= 0.5),
        })
    ctx.csv("e4b_time_headtohead", pl.DataFrame(time_rows))
    ctx.csv("e4b_memory_headtohead", pl.DataFrame(mem_rows))
    for src in ("mcc-small", "mcc-large"):
        if any(k[0] == src for k in time_pts):
            _scatter(ctx, src, time_pts, cap)
            _memory_scatter(ctx, src, mem_pts)


def _grid(p, rows: int, cols: int):
    return p.subplots(rows, cols, figsize=(3.2 * cols, 3.1 * rows), squeeze=False, sharex=True, sharey=True, constrained_layout=True)


def _scatter(ctx: Ctx, src: str, pts: dict, cap: float) -> None:
    p = plt()
    from matplotlib.lines import Line2D
    allt = [v for (s, *_), l in pts.items() if s == src for a, b, _ in l for v in (a, b) if v > 0]
    lo, hi = min(allt) / 2, cap * 3
    fig, axs = _grid(p, len(REFERENCES), len(MCC_QUESTIONS))
    used: set[str] = set()
    for r, ref in enumerate(REFERENCES):
        for c, q in enumerate(MCC_QUESTIONS):
            ax = axs[r][c]
            ax.set_autoscale_on(False)
            for key, _, color in OUTCOMES:
                v = [(a, b) for a, b, o in pts.get((src, ref, q), []) if o == key]
                if v:
                    used.add(key)
                    if key == "neither":
                        ax.scatter([a for a, _ in v], [b for _, b in v], s=34, color=color, linewidths=1.2, marker="x", zorder=5)
                    else:
                        ax.scatter([a for a, _ in v], [b for _, b in v], s=14, color=color, alpha=0.6, linewidths=0)
            ax.plot([lo, hi], [lo, hi], color="#777777", lw=0.9)
            ax.axhline(cap, color="#999999", ls="--", lw=0.8)
            ax.axvline(cap, color="#999999", ls="--", lw=0.8)
            ax.set_xscale("log"); ax.set_yscale("log"); ax.set_xlim(lo, hi); ax.set_ylim(lo, hi)
            ax.set_title(f"{QUERY_LABEL[q]}", fontsize=8.5, loc="left")
            ax.tick_params(labelsize=7)
    for ax in axs[-1]:
        ax.set_xlabel("time with petrivet (ms, log scale)", fontsize=8)
    for r, ref in enumerate(REFERENCES):
        axs[r][0].set_ylabel(f"time with {TOOL_NAME[ref]} (ms, log scale)", fontsize=8)
    fig.legend(handles=[Line2D([], [], marker="x" if k == "neither" else "o", color=col, ls="none", label=label) for k, label, col in OUTCOMES if k in used],
               loc="outside lower center", ncol=4, frameon=False, fontsize=8)
    save_svg(fig, ctx.fig_path(f"e4b_time_scatter_{src.replace('-', '_')}"))


def _memory_scatter(ctx: Ctx, src: str, pts: dict) -> None:
    """Memory of petrivet (x) against the reference (y), MiB. Filled: both answered; ringed: a memory-out; hollow: a run stopped
    by the time limit, whose value is the memory when it was killed."""
    p = plt()
    from matplotlib.lines import Line2D
    vals = [v for (s, *_), l in pts.items() if s == src for a, b, *_ in l for v in (a, b) if v]
    if not vals:
        return
    lo, hi = min(vals) / 1.5, max(vals) * 2
    fig, axs = _grid(p, len(REFERENCES), len(MCC_QUESTIONS))
    for r, ref in enumerate(REFERENCES):
        for c, q in enumerate(MCC_QUESTIONS):
            ax = axs[r][c]
            ax.set_autoscale_on(False)
            for a, b, sa, sb in pts.get((src, ref, q), []):
                if a is None or b is None:
                    continue
                if "memory-out" in (sa, sb):
                    kw = dict(mfc="#0072B2", mec="#000000", mew=1.0)
                elif "time-out" in (sa, sb):
                    kw = dict(mfc="white", mec="#0072B2", mew=0.9)
                else:
                    kw = dict(mfc="#0072B2", mec="#0072B2", mew=0.0, alpha=0.6)
                ax.plot([a], [b], marker="o", ms=3.6, ls="none", **kw)
            ax.plot([lo, hi], [lo, hi], color="#777777", lw=0.9)
            ax.set_xscale("log"); ax.set_yscale("log"); ax.set_xlim(lo, hi); ax.set_ylim(lo, hi)
            ax.set_title(f"{QUERY_LABEL[q]}", fontsize=8.5, loc="left")
            ax.tick_params(labelsize=7)
    for ax in axs[-1]:
        ax.set_xlabel("memory of petrivet (MiB, log scale)", fontsize=8)
    for r, ref in enumerate(REFERENCES):
        axs[r][0].set_ylabel(f"memory of {TOOL_NAME[ref]} (MiB, log scale)", fontsize=8)
    fig.legend(handles=[Line2D([], [], marker="o", color="#0072B2", ls="none", ms=5, label="answered"),
                        Line2D([], [], marker="o", mfc="#0072B2", mec="#000000", color="#0072B2", ls="none", ms=5, label="ended by the memory limit"),
                        Line2D([], [], marker="o", mfc="white", mec="#0072B2", color="#0072B2", ls="none", ms=5, label="stopped by the time limit (memory when stopped)")],
               loc="outside lower center", ncol=3, frameon=False, fontsize=8)
    save_svg(fig, ctx.fig_path(f"e4b_memory_scatter_{src.replace('-', '_')}"))


# ------------------------------------------------------------------------------------------------- scaling families

def _fam(ctx: Ctx, t: pl.DataFrame, cap: float) -> None:
    f = t.filter((pl.col("source") == "fam") & pl.col("solver_id").is_in(list(FAM_TOOLS)) & pl.col("size").is_not_null())
    if f.height == 0:
        ctx.skip("e4b_fam", "no scaling-family cells (is the manifest given?)")
        return
    rows = []
    for (fam, cls, size, tool), g in f.group_by("family", "cls", "size", "solver_id", maintain_order=True):
        times = [r["wall_ms"] if r["solved"] else cap for r in g.iter_rows(named=True)]
        mems = [_memory(r) for r in g.iter_rows(named=True)]
        known = [m for m, _ in mems if m is not None]
        rows.append({
            "family": fam, "cls": cls, "size": int(size), "solver_id": tool, "n_cells": g.height, "total_ms": float(sum(times)),
            "n_failed": int((~g["solved"]).sum()), "max_rss_mib": max(known) if known else None,
            "n_memout": sum(1 for _, s in mems if s == "memory-out"), "n_timeout_value": sum(1 for m, s in mems if s == "time-out" and m is not None),
        })
    df = pl.DataFrame(rows, infer_schema_length=None).with_columns(pl.col("family").map_elements(family_rank, return_dtype=pl.Int64).alias("_rank")).sort("_rank", "family", "solver_id", "size").drop("_rank")
    ctx.csv("e4b_scaling_fam", df)
    fr = []
    for (fam, tool), g in df.group_by("family", "solver_id", maintain_order=True):
        ok = g.filter(pl.col("n_failed") == 0)["size"].to_list()
        fr.append({"family": fam, "solver_id": tool, "max_size_all_solved": max(ok) if ok else None, "max_size_attempted": int(g["size"].max())})
    ctx.csv("e4b_frontier", pl.DataFrame(fr, infer_schema_length=None).with_columns(pl.col("family").map_elements(family_rank, return_dtype=pl.Int64).alias("_rank")).sort("_rank", "family", "solver_id").drop("_rank"))
    _scaling_fig(ctx, df, "total_ms", "e4b_scaling_time_fam", "time for four questions (ms, log scale)", cap)
    _scaling_fig(ctx, df, "max_rss_mib", "e4b_scaling_memory_fam", "largest peak memory (MiB, log scale)", None)


def _scaling_fig(ctx: Ctx, df: pl.DataFrame, col: str, name: str, ylabel: str, cap_line: float | None) -> None:
    p = plt()
    from matplotlib.lines import Line2D
    order = {c: i for i, c in enumerate(CLASS_ORDER)}
    fams = sorted(df.group_by("family", "cls").len().iter_rows(named=True), key=lambda r: (order.get(r["cls"], 99), family_rank(r["family"]), r["family"]))
    n = len(fams)
    cols = min(3, n)
    nrows = (n + cols - 1) // cols
    fig, axs = p.subplots(nrows, cols, figsize=(3.7 * cols, 2.6 * nrows + 0.4), squeeze=False, sharex=True, sharey=True, constrained_layout=True)
    gib = ctx.cfg.get("run", {}).get("mem_limit_gib")
    for ax, fr in zip(axs.ravel(), fams):
        d = df.filter(pl.col("family") == fr["family"])
        for tool in FAM_TOOLS:
            dt = d.filter((pl.col("solver_id") == tool) & pl.col(col).is_not_null()).sort("size")
            if dt.height == 0:
                continue
            color = SOLVER_COLOR[tool]
            ax.plot(dt["size"], dt[col], color=color, lw=0.9, alpha=0.8, zorder=2)
            for r in dt.iter_rows(named=True):
                stopped = r["n_failed"] > 0
                ring = col == "max_rss_mib" and r["n_memout"] > 0
                ax.plot([r["size"]], [r[col]], marker="o", ms=4.2, ls="none", mfc="white" if stopped and not ring else color, mec="#000000" if ring else color, mew=1.0, zorder=3)
        if cap_line is not None:
            ax.axhline(cap_line * 4, color="#999999", ls="--", lw=0.8)  # four questions, each at the cap
        elif gib:
            ax.axhline(float(gib) * 1024, color="#999999", ls="--", lw=0.8)
        ax.set_xscale("log", base=2); ax.set_yscale("log")
        ax.set_title(FAMILY_LABEL.get(fr["family"], fr["family"]), fontsize=8, loc="left")
        ax.tick_params(labelsize=7)
    for ax in axs.ravel()[n:]:
        ax.axis("off")
    for ax in axs[-1]:
        ax.set_xlabel("size parameter n of the family (log scale)", fontsize=8)
    for ax in axs[:, 0]:
        ax.set_ylabel(ylabel, fontsize=8)
    handles = [Line2D([], [], color=SOLVER_COLOR[tool], lw=1.5, label=solver_label(tool)) for tool in FAM_TOOLS]
    handles += [Line2D([], [], marker="o", color="#555555", ls="none", ms=5, label="all four questions answered"),
                Line2D([], [], marker="o", mfc="white", mec="#555555", color="#555555", ls="none", ms=5, label="some question not answered (a bound)")]
    if col == "max_rss_mib":
        handles.append(Line2D([], [], marker="o", mfc="#555555", mec="#000000", color="#555555", ls="none", ms=5, label="some question ended by the memory limit"))
    fig.legend(handles=handles, loc="outside upper center", ncol=3, frameon=False, fontsize=7.5)
    save_svg(fig, ctx.fig_path(name))
