"""Per (instance, query, target, solver) cells of campaign B for the comparison analyses (E4, E5b)."""

from __future__ import annotations

import polars as pl

from .common import Ctx


def cell_table(ctx: Ctx, campaign: str = "B") -> pl.DataFrame:
    """Columns: source, cls, instance_id, query, tid, solver_id, verdict (rep 0), outcome (vs truth), wall_ms (median over
    repetitions), rss_kib (median), truth_bool, solved (definite and not DISAGREE), par2_ms (wall if solved within the
    cap, else 2 x cap). The cap is ``ctx.cap()``; without one PAR-2 is null.

    Only the first repetition's verdict counts for classification; timings are the median of all repetitions
    (repetitions are only run for TRUE/FALSE/COUNT/UNKNOWN, a TIMEOUT cell has one run).
    """
    r = ctx.runs_meta(campaign)
    agg = r.group_by("source", "cls", "instance_id", "query", "tid", "solver_id").agg(
        pl.col("verdict").sort_by("rep").first().alias("verdict"),
        pl.col("wall_ms").median().alias("wall_ms"),
        pl.col("peak_rss_kib").median().alias("rss_kib"),
        pl.col("limit_ms").max().alias("limit_ms"),
    )
    adj = ctx.adj.filter(pl.col("campaign") == campaign).select("instance_id", "query", "tid", "solver_id", "outcome", "truth_bool", "truth_source")
    t = agg.join(adj, on=["instance_id", "query", "tid", "solver_id"], how="left")
    t = t.with_columns(
        (pl.col("verdict").is_in(["TRUE", "FALSE", "COUNT"]) & pl.col("outcome").ne("DISAGREE").fill_null(True)).alias("solved")
    )
    cap = ctx.cap()
    if cap is not None:
        t = t.with_columns(pl.when(pl.col("solved") & (pl.col("wall_ms") <= cap)).then(pl.col("wall_ms")).otherwise(2 * cap).alias("par2_ms"))
    else:
        t = t.with_columns(pl.lit(None, dtype=pl.Float64).alias("par2_ms"))
    return t.sort("source", "instance_id", "query", "tid", "solver_id")
