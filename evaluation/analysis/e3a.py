"""E3a: cost control by effort: what `pv-poly` (efficient effort only) answers.

Over campaign B (rep 0), solver ``pv-poly``, per source (``mcc-large`` is exploratory, flagged in the CSV):
  n_runs      applicable runs = all runs except NOT_SUPPORTED, INPUT_REJECTED, RUNNER_ERROR, OUT_OF_SCOPE
  n_definite  verdict TRUE/FALSE/COUNT;  n_correct = AGREE;  n_wrong = DISAGREE;  n_unknown = UNKNOWN
  share_definite, share_correct, share_unknown = counts / n_runs
  n_poly_answers_general_timeout  cells where pv-poly is AGREE (definite and correct) and pv-general is TIMEOUT/MEMOUT
Keys: e3a.<source>.<field>; list of the H3a instances in e3a_h3a_instances.csv.
"""

from __future__ import annotations

import polars as pl

from .common import Ctx

POLY, GENERAL = "pv-poly", "pv-general"
EXPLORATORY = {"mcc-large"}


def run(ctx: Ctx) -> None:
    adj = ctx.adj.filter((pl.col("campaign") == "B"))
    poly = adj.filter(pl.col("solver_id") == POLY)
    if poly.height == 0:
        ctx.skip("e3a", "no pv-poly runs in campaign B")
        return
    app = poly.filter(~pl.col("outcome").is_in(["NOT_SUPPORTED", "INPUT_REJECTED", "RUNNER_ERROR", "OUT_OF_SCOPE"]))
    gen = adj.filter(pl.col("solver_id") == GENERAL).select("instance_id", "query", "tid", pl.col("verdict").alias("general_verdict"), pl.col("wall_ms").alias("general_wall_ms"))
    j = poly.join(gen, on=["instance_id", "query", "tid"], how="left")
    h3a = j.filter((pl.col("outcome") == "AGREE") & pl.col("general_verdict").is_in(["TIMEOUT", "MEMOUT"]))
    ctx.csv("e3a_h3a_instances", h3a.select("source", "cls", "instance_id", "query", "tid", "verdict", pl.col("wall_ms").alias("poly_wall_ms"), "general_verdict", "general_wall_ms").sort("source", "instance_id", "query", "tid"))

    rows = []
    for (src,), g in app.group_by("source"):
        n = g.height
        d = g.filter(pl.col("verdict").is_in(["TRUE", "FALSE", "COUNT"])).height
        c = g.filter(pl.col("outcome") == "AGREE").height
        w = g.filter(pl.col("outcome") == "DISAGREE").height
        u = g.filter(pl.col("verdict") == "UNKNOWN").height
        hg = h3a.filter(pl.col("source") == src).height
        rows.append({"source": src, "exploratory": src in EXPLORATORY, "n_runs": n, "n_definite": d, "n_correct": c, "n_wrong": w, "n_unknown": u,
                     "share_definite": d / n if n else None, "share_correct": c / n if n else None, "share_unknown": u / n if n else None,
                     "n_poly_answers_general_timeout": hg})
    tab = pl.DataFrame(rows).sort("source")
    ctx.csv("e3a", tab)
    for r in rows:
        for f in ("n_runs", "n_definite", "n_correct", "n_wrong", "n_unknown", "share_definite", "share_correct", "share_unknown", "n_poly_answers_general_timeout"):
            ctx.num("e3a", r["source"], f, r[f])
    ctx.num("e3a", "n_h3a_instances", h3a.select("instance_id").unique().height)
    ctx.num("e3a", "h3a_supported", h3a.height > 0)
