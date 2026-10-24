"""E8: ITS-Tools, deadlock-freedom (Q1) with a 90 s cap against the 30 s cap of campaign B.

ITS-Tools spends a fixed internal wait of about 30 s in some strategies, whatever its own timeout says, so at the
30 s cap of the campaigns some of its answers are cut off just after that wait (SPEC amendment on ITS-Tools). The
sensitivity run repeats the Q1 cells of ITS-Tools with a 90 s cap into ``results/its-q1-90s/runs.jsonl`` (an optional
input: without the file nothing is computed and no key is written except ``e8.present``). For the Q1 cells that ended in
TIMEOUT in campaign B it counts how many are answered with 90 s and when (about 30 to 35 s: the fixed wait; later: real
work), and checks the answers of the 90 s run against the truth (cells of unbounded nets are out of scope and counted apart)
and against the 30 s run.

Keys: e8.present, e8.cap_ms, e8.n_cells, e8.n_timeouts_30s, e8.n_answered_90s, e8.n_answered_30_35s,
e8.n_answered_35_60s, e8.n_answered_60s_plus, e8.n_still_timeout, e8.answer_median_s, e8.answer_max_s, e8.n_flips,
e8.n_truth_agree, e8.n_truth_disagree, e8.n_unbounded_skipped
Table: e8_its_q1_90s.csv (one row)
"""

from __future__ import annotations

import polars as pl

from .common import Ctx, load_runs

SOLVER = "its-tools"
SENS_DIR = "its-q1-90s"


def run(ctx: Ctx) -> None:
    path = ctx.results / SENS_DIR / "runs.jsonl"
    if not path.is_file():
        ctx.numbers["e8.present"] = False
        return
    ctx.numbers["e8.present"] = True
    base = ctx.runs.filter((pl.col("campaign") == "B") & (pl.col("solver_id") == SOLVER) & (pl.col("query") == "Q1") & (pl.col("rep") == 0)
                           & (pl.col("instance_id") != "BASELINE"))
    sens = load_runs(path)[0].filter((pl.col("solver_id") == SOLVER) & (pl.col("query") == "Q1") & (pl.col("rep") == 0) & (pl.col("instance_id") != "BASELINE"))
    if base.height == 0 or sens.height == 0:
        ctx.skip("e8", "no ITS-Tools Q1 runs in campaign B or in the sensitivity run")
        return
    j = base.select("instance_id", pl.col("verdict").alias("v30")).join(
        sens.select("instance_id", pl.col("verdict").alias("v90"), pl.col("wall_ms").alias("wall90"), pl.col("limit_ms").alias("cap90")), on="instance_id", how="inner")
    definite = ["TRUE", "FALSE"]
    slow = j.filter(pl.col("v30") == "TIMEOUT")
    answered = slow.filter(pl.col("v90").is_in(definite))
    secs = answered["wall90"] / 1000.0
    n_ans = answered.height
    flips = j.filter(pl.col("v30").is_in(definite) & pl.col("v90").is_in(definite) & (pl.col("v30") != pl.col("v90"))).height
    # truth: a boolean Q1 truth; cells of unbounded nets (Q2 truth false) are out of scope
    unbounded = set(ctx.truth.filter((pl.col("query") == "Q2") & (pl.col("truth_bool") == False))["instance_id"].to_list())  # noqa: E712
    t1 = ctx.truth.filter((pl.col("query") == "Q1") & pl.col("truth_bool").is_not_null()).select("instance_id", "truth_bool")
    k = sens.filter(pl.col("verdict").is_in(definite)).select("instance_id", "verdict").join(t1, on="instance_id", how="inner")
    k_in = k.filter(~pl.col("instance_id").is_in(list(unbounded)))
    agree = k_in.filter((pl.col("verdict") == "TRUE") == pl.col("truth_bool")).height
    row = {
        "cap_ms": float(j["cap90"].max()), "n_cells": j.height, "n_timeouts_30s": slow.height, "n_answered_90s": n_ans,
        "n_answered_30_35s": int(((secs >= 30) & (secs < 35)).sum()), "n_answered_35_60s": int(((secs >= 35) & (secs < 60)).sum()),
        "n_answered_60s_plus": int((secs >= 60).sum()), "n_still_timeout": slow.height - n_ans,
        "answer_median_s": float(secs.median()) if n_ans else None, "answer_max_s": float(secs.max()) if n_ans else None,
        "n_flips": flips, "n_truth_agree": agree, "n_truth_disagree": k_in.height - agree, "n_unbounded_skipped": k.height - k_in.height,
    }
    for key, v in row.items():
        ctx.num("e8", key, v)
    ctx.csv("e8_its_q1_90s", pl.DataFrame([row]))
