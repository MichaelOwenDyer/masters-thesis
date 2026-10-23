"""E5a: QR/QC correctness on ``gen``: agreement with truth, split yes (truth TRUE) / no (truth FALSE).

Same counting as E1 (conclusive = AGREE + DISAGREE). Keys (all solvers; solver names normalised, e.g. pv_dispatch):
  e5a.<solver>.<qr|qc>.<yes|no>.<field>, e5a.<solver>.<qr|qc>.all.<field>   field as in E1
"""

from __future__ import annotations

import polars as pl

from .common import Ctx
from .e1 import agreement_table, with_truth_only

SOURCE = "gen"


def run(ctx: Ctx) -> None:
    adj = ctx.adj.filter(pl.col("query").is_in(["QR", "QC"]) & (pl.col("source") == SOURCE))
    if adj.height == 0:
        ctx.skip("e5a", "no QR/QC runs on gen in campaigns A/B")
        return
    adj = with_truth_only(adj).with_columns(
        pl.when(pl.col("truth_bool") == True).then(pl.lit("yes")).when(pl.col("truth_bool") == False).then(pl.lit("no")).otherwise(pl.lit("unlabelled")).alias("stratum")  # noqa: E712
    )
    # non-answer runs have a truth value in the stratum only if the cell was labelled
    tab = agreement_table(adj, ["solver_id", "query", "stratum"])
    ctx.csv("e5a", tab)
    tab_cls = agreement_table(adj, ["solver_id", "cls", "query", "stratum"])
    ctx.csv("e5a_by_class", tab_cls)
    tot = agreement_table(adj, ["solver_id", "query"])
    for t, st in ((tab, None), (tot, "all")):
        for r in t.iter_rows(named=True):
            for f in ("n_conclusive", "n_agree", "n_disagree", "n_inconclusive", "inconclusive_rate", "cp_upper_95"):
                ctx.num("e5a", r["solver_id"], r["query"], st or r["stratum"], f, r[f])
