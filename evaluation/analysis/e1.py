"""E1: agreement with truth, per solver x source x class x query (SPEC 11, E1).

conclusive = runs with a definite answer and a truth value (AGREE + DISAGREE).
inconclusive = runs that have a truth value but no definite answer (UNKNOWN, TIMEOUT, MEMOUT, CRASH, INPUT_REJECTED,
NOT_SUPPORTED, RUNNER_ERROR, SHARD_FAILED); inconclusive_rate = n_inconclusive / (n_conclusive + n_inconclusive).
OUT_OF_SCOPE (Q1/Q3/Q4 on a net with Q2 = false) and NO_TRUTH are reported as separate counts and in no rate.

numbers.json keys (subject solver only, sources never pooled):
  e1.<source>.<field>, e1.<source>.<query>.<field>, e1.<source>.<class>.<query>.<field>
  with field in n_conclusive n_agree n_disagree n_inconclusive inconclusive_rate cp_upper_95
  e1.n_crash, e1.n_input_rejected (subject), e1.rep_verdict_differs, e1.adjudication_crosscheck.*
"""

from __future__ import annotations

import polars as pl

from .common import Ctx, key
from .stats import cp_upper

NON_ANSWER = ("UNKNOWN", "TIMEOUT", "MEMOUT", "CRASH", "INPUT_REJECTED", "NOT_SUPPORTED", "RUNNER_ERROR", "SHARD_FAILED")


def agreement_table(adj: pl.DataFrame, by: list[str]) -> pl.DataFrame:
    """One row per group with the E1 counts and the Clopper-Pearson bound."""
    g = adj.group_by(by).agg(
        (pl.col("outcome") == "AGREE").sum().alias("n_agree"),
        (pl.col("outcome") == "DISAGREE").sum().alias("n_disagree"),
        pl.col("outcome").is_in(list(NON_ANSWER)).sum().alias("n_inconclusive"),
        (pl.col("outcome") == "NO_TRUTH").sum().alias("n_no_truth"),
        (pl.col("outcome") == "OUT_OF_SCOPE").sum().alias("n_out_of_scope"),
    ).sort(by)
    g = g.with_columns((pl.col("n_agree") + pl.col("n_disagree")).alias("n_conclusive"))
    return g.with_columns(
        pl.struct("n_conclusive", "n_disagree").map_elements(
            lambda s: cp_upper(0, s["n_conclusive"]) if s["n_disagree"] == 0 else None, return_dtype=pl.Float64
        ).alias("cp_upper_95"),
        pl.when(pl.col("n_conclusive") + pl.col("n_inconclusive") > 0)
        .then(pl.col("n_inconclusive") / (pl.col("n_conclusive") + pl.col("n_inconclusive"))).otherwise(None).alias("inconclusive_rate"),
    )


def with_truth_only(adj: pl.DataFrame) -> pl.DataFrame:
    """Non-answer runs only count as inconclusive when a truth value exists for the cell."""
    return adj.with_columns(
        pl.when(pl.col("outcome").is_in(list(NON_ANSWER)) & pl.col("truth_kind").is_null()).then(pl.lit("NO_TRUTH_NONANSWER")).otherwise(pl.col("outcome")).alias("outcome")
    )


def emit(ctx: Ctx, prefix: str, tab: pl.DataFrame, parts_of) -> None:
    for r in tab.iter_rows(named=True):
        for f in ("n_conclusive", "n_agree", "n_disagree", "n_inconclusive", "inconclusive_rate", "cp_upper_95"):
            ctx.num(prefix, *parts_of(r), f, r[f])


def run(ctx: Ctx) -> None:
    adj = ctx.adj
    if adj.height == 0:
        ctx.skip("e1", "no runs of campaigns A or B")
        return
    if ctx.truth.height == 0:
        ctx.skip("e1", "truth file is empty")
        return
    adj = with_truth_only(adj)
    tab = agreement_table(adj, ["solver_id", "source", "cls", "query"])
    ctx.csv("e1", tab)
    sub = adj.filter(pl.col("solver_id") == ctx.subject)
    if sub.height == 0:
        ctx.skip("e1_numbers", f"no runs of subject solver {ctx.subject}")
    else:
        by_sq = agreement_table(sub, ["source", "query"])
        by_s = agreement_table(sub, ["source"])
        by_scq = agreement_table(sub, ["source", "cls", "query"])
        emit(ctx, "e1", by_s, lambda r: (r["source"],))
        emit(ctx, "e1", by_sq, lambda r: (r["source"], r["query"]))
        emit(ctx, "e1", by_scq, lambda r: (r["source"], r["cls"], r["query"]))
        ctx.num("e1", "n_crash", sub.filter(pl.col("outcome") == "CRASH").height)
        ctx.num("e1", "n_input_rejected", sub.filter(pl.col("outcome") == "INPUT_REJECTED").height)
        for r in sub.group_by("source").agg((pl.col("outcome") == "CRASH").sum().alias("c"), (pl.col("outcome") == "INPUT_REJECTED").sum().alias("i")).iter_rows(named=True):
            ctx.num("e1", r["source"], "n_crash", r["c"])
            ctx.num("e1", r["source"], "n_input_rejected", r["i"])
    ctx.num("e1", "rep_verdict_differs", ctx.runs.filter(pl.col("verdict_differs")).height)

    # disagreements (all solvers) with pattern from adjudication.jsonl when available
    dis = adj.filter(pl.col("outcome") == "DISAGREE").select("run_id", "campaign", "instance_id", "query", "tid", "solver_id", "source", "cls", "verdict", "truth_source")
    if ctx.adj_rows:
        pat = {r.get("run_id"): r.get("pattern") for r in ctx.adj_rows}
        dis = dis.with_columns(pl.col("run_id").map_elements(lambda x: pat.get(x), return_dtype=pl.Utf8).alias("pattern"))
    ctx.csv("e1_disagreements", dis)
    ctx.num("e1", "n_disagree_total_subject", dis.filter(pl.col("solver_id") == ctx.subject).height)

    # truth exclusions (truth_excluded.jsonl, written by the truth stage)
    if ctx.truth_excluded:
        ex = pl.DataFrame([{"corpus_source": r.get("corpus_source"), "cls": str(r.get("class") or "unknown").lower(), "query": r.get("query"), "reason": r.get("reason")} for r in ctx.truth_excluded])
        tab_ex = ex.group_by("corpus_source", "cls", "query", "reason").len().rename({"len": "n_excluded"}).sort("corpus_source", "cls", "query", "reason")
        ctx.csv("e1_truth_exclusions", tab_ex)
        ctx.num("e1", "n_truth_excluded", ex.height)
    else:
        ctx.numbers["e1.truth_exclusions_note"] = "results/truth_excluded.jsonl absent or empty"

    # cross-check against results/adjudication.jsonl (written by `petrivet-eval adjudicate`)
    if ctx.adj_rows is not None:
        theirs = {r.get("run_id"): r.get("outcome") for r in ctx.adj_rows}
        mine = dict(zip(adj["run_id"].to_list(), adj["outcome"].to_list()))
        common = [k for k in mine if k in theirs]
        diff = [k for k in common if mine[k] != theirs[k] and not (mine[k] == "NO_TRUTH_NONANSWER")]
        ctx.num("e1", "adjudication_crosscheck", "n_compared", len(common))
        ctx.num("e1", "adjudication_crosscheck", "n_outcome_differs", len(diff))
        ctx.num("e1", "adjudication_crosscheck", "n_only_in_runs", len([k for k in mine if k not in theirs]))
