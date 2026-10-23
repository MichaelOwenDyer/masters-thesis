"""E5b: QR/QC comparison on ``mcc-small`` targets (campaign B): solved count and time per solver.

Limitation to print next to every table (SPEC 5.5): labels exist only for targets that LoLA and TINA agreed on, so the
set is biased towards decidable, easier targets; correctness of "no" labels is agreement of two tools, not a proof.
Fields as in E4 (n_cells, n_solved, par2_ms, median_solved_ms), split by truth stratum yes/no.
Keys: e5b.<solver>.<qr|qc>.<yes|no|all>.<field>
"""

from __future__ import annotations

import polars as pl

from .cells import cell_table
from .common import Ctx
from .e4 import summarise

SOURCE = "mcc-small"


def run(ctx: Ctx) -> None:
    t = cell_table(ctx, "B").filter(pl.col("query").is_in(["QR", "QC"]) & (pl.col("source") == SOURCE))
    if t.height == 0:
        ctx.skip("e5b", "no QR/QC runs on mcc-small in campaign B")
        return
    cap = ctx.cap()
    t = t.with_columns(pl.when(pl.col("truth_bool") == True).then(pl.lit("yes")).when(pl.col("truth_bool") == False).then(pl.lit("no")).otherwise(pl.lit("unlabelled")).alias("stratum"))  # noqa: E712
    # stratum of a cell is a property of the cell, not of the solver: take it from any solver that has truth
    strat = t.filter(pl.col("stratum") != "unlabelled").select("instance_id", "query", "tid", pl.col("stratum").alias("cell_stratum")).unique(subset=["instance_id", "query", "tid"])
    t = t.join(strat, on=["instance_id", "query", "tid"], how="left").with_columns(pl.col("cell_stratum").fill_null("unlabelled"))
    rows = []
    for q in ["QR", "QC"]:
        for st in ["yes", "no", "all"]:
            tq = t.filter(pl.col("query") == q)
            if st != "all":
                tq = tq.filter(pl.col("cell_stratum") == st)
            n_cells = tq.select("instance_id", "tid").unique().height
            for s in sorted(t["solver_id"].unique().to_list()):
                d = summarise(tq.filter(pl.col("solver_id") == s), [], n_cells, cap)
                rows.append({"solver_id": s, "query": q, "stratum": st, **d})
                for f, v in d.items():
                    ctx.num("e5b", s, q, st, f, v)
    ctx.csv("e5b", pl.DataFrame(rows))
