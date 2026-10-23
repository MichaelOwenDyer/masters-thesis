"""E5a: QR/QC correctness on ``gen``: agreement with truth, split yes (truth TRUE) / no (truth FALSE).

Same counting as E1 (conclusive = AGREE + DISAGREE). Keys (all solvers; solver names normalised, e.g. pv_dispatch):
  e5a.<solver>.<qr|qc>.<yes|no>.<field>, e5a.<solver>.<qr|qc>.all.<field>   field as in E1
  labels resting on LoLA alone (truth source ``lola-only``) get a ``lola_only`` segment before the field:
  e5a.<solver>.<qr|qc>.<yes|no|all>.lola_only.<field>; they are never pooled with the two-tool labels
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
    # labels that rest on LoLA alone (TINA skipped on unbounded nets) are their own tier, never pooled with the two-tool labels
    adj = adj.with_columns(pl.when(pl.col("truth_source") == "lola-only").then(pl.lit("lola_only")).otherwise(pl.lit("two_tool")).alias("tier"))
    # non-answer runs have a truth value in the stratum only if the cell was labelled
    tab = agreement_table(adj, ["solver_id", "query", "tier", "stratum"])
    ctx.csv("e5a", tab)
    tab_cls = agreement_table(adj, ["solver_id", "cls", "query", "tier", "stratum"])
    ctx.csv("e5a_by_class", tab_cls)
    tot = agreement_table(adj, ["solver_id", "query", "tier"])
    for t, st in ((tab, None), (tot, "all")):
        for r in t.iter_rows(named=True):
            # two-tool keys keep their names; the single-tool tier gets a `lola_only` segment
            seg = ("lola_only",) if r["tier"] == "lola_only" else ()
            for f in ("n_conclusive", "n_agree", "n_disagree", "n_inconclusive", "inconclusive_rate", "cp_upper_95"):
                ctx.num("e5a", r["solver_id"], r["query"], st or r["stratum"], *seg, f, r[f])
