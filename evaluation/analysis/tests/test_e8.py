import json

import polars as pl

from analysis import e8
from analysis.common import RUN_SCHEMA, frame


def run_row(campaign, inst, verdict, wall, limit, solver="its-tools", query="Q1"):
    return {"run_id": f"{campaign}-{inst}-{solver}", "campaign": campaign, "instance_id": inst, "query": query, "target_id": None,
            "solver_id": solver, "rep": 0, "verdict": verdict, "wall_ms": wall, "limit_ms": limit}


class Stub:
    """The part of Ctx that e8 uses."""

    def __init__(self, results, runs, truth):
        self.results, self.numbers, self.skipped, self.tables = results, {}, [], {}
        self.runs = runs
        self.truth = truth

    def num(self, *parts):
        *key, v = parts
        self.numbers[".".join(key)] = v

    def csv(self, name, df):
        self.tables[name] = df

    def skip(self, name, why):
        self.skipped.append((name, why))


def truth_frame(rows):
    return pl.DataFrame(rows, schema={"instance_id": pl.Utf8, "query": pl.Utf8, "truth_bool": pl.Boolean})


def test_without_the_sensitivity_run_nothing_is_computed(tmp_path):
    ctx = Stub(tmp_path, frame([], RUN_SCHEMA).with_columns(pl.col("target_id").fill_null("").alias("tid")), truth_frame([]))
    e8.run(ctx)
    assert ctx.numbers == {"e8.present": False} and not ctx.skipped


def test_timeouts_are_answered_with_the_longer_cap(tmp_path):
    base = [
        run_row("B", "i1", "TIMEOUT", 30003, 30000), run_row("B", "i2", "TIMEOUT", 30003, 30000), run_row("B", "i3", "TIMEOUT", 30003, 30000),
        run_row("B", "i4", "TRUE", 1200, 30000), run_row("B", "i5", "TIMEOUT", 30003, 30000),
        run_row("B", "BASELINE", "TRUE", 900, 30000), run_row("B", "i1", "TRUE", 5, 30000, solver="lola"),
    ]
    sens = [
        run_row("B", "i1", "TRUE", 31100, 90000), run_row("B", "i2", "TRUE", 65000, 90000), run_row("B", "i3", "TIMEOUT", 90003, 90000),
        run_row("B", "i4", "TRUE", 1000, 90000), run_row("B", "i5", "FALSE", 31300, 90000),
    ]
    (tmp_path / "its-q1-90s").mkdir()
    (tmp_path / "its-q1-90s" / "runs.jsonl").write_text("".join(json.dumps(r) + "\n" for r in sens))
    runs = frame(base, RUN_SCHEMA).with_columns(pl.col("target_id").fill_null("").alias("tid"))
    truth = truth_frame([
        {"instance_id": i, "query": "Q1", "truth_bool": True} for i in ("i1", "i2", "i3", "i4", "i5")
    ] + [{"instance_id": "i5", "query": "Q2", "truth_bool": False}])
    ctx = Stub(tmp_path, runs, truth)
    e8.run(ctx)
    n = ctx.numbers
    assert (n["e8.n_cells"], n["e8.n_timeouts_30s"], n["e8.n_answered_90s"], n["e8.n_still_timeout"]) == (5, 4, 3, 1)
    assert (n["e8.n_answered_30_35s"], n["e8.n_answered_35_60s"], n["e8.n_answered_60s_plus"]) == (2, 0, 1)
    # i5 is unbounded: its FALSE (against a truth of TRUE) is out of scope and counted apart
    assert (n["e8.n_truth_agree"], n["e8.n_truth_disagree"], n["e8.n_unbounded_skipped"]) == (3, 0, 1)
    assert n["e8.n_flips"] == 0 and n["e8.cap_ms"] == 90000.0 and abs(n["e8.answer_median_s"] - 31.3) < 1e-9
    assert "e8_its_q1_90s" in ctx.tables
