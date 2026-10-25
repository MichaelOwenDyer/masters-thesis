import gzip
import json

import polars as pl

from analysis import e0, e2b, e9
from analysis.common import RUN_SCHEMA, frame, key, with_meta


class Stub:
    def __init__(self, results, runs=None, meta=None, manifests=()):
        self.results, self.numbers, self.skipped, self.tables = results, {}, [], {}
        self.runs, self.meta, self.manifests = runs, meta, list(manifests)

    def num(self, *parts):
        *parts, v = parts
        self.numbers[key(*parts)] = v

    def csv(self, name, df):
        self.tables[name] = df

    def skip(self, name, why):
        self.skipped.append((name, why))

    def runs_meta(self, campaign=None):
        r = self.runs.filter(pl.col("instance_id") != "BASELINE") if False else self.runs
        r = r.filter(pl.col("campaign") == campaign) if campaign else r
        return with_meta(r.filter(pl.col("instance_id") != "BASELINE"), self.meta)


def run_row(inst, solver, verdict, wall, query="Q3", rep=0, campaign="B"):
    return {"run_id": f"{campaign}-{inst}-{solver}-{query}-{rep}", "campaign": campaign, "instance_id": inst, "query": query, "target_id": None,
            "solver_id": solver, "rep": rep, "verdict": verdict, "wall_ms": wall, "limit_ms": 30000, "peak_rss_kib": 1000}


def runs_frame(rows):
    return frame(rows, RUN_SCHEMA).with_columns(pl.col("target_id").fill_null("").alias("tid"))


def meta_frame(rows):
    schema = {"instance_id": pl.Utf8, "source": pl.Utf8, "cls": pl.Utf8, "family": pl.Utf8, "size": pl.Float64, "places": pl.Float64, "transitions": pl.Float64}
    return pl.DataFrame(rows, schema=schema)


def test_e0_counts_manifest_lines_and_duplicates(tmp_path):
    (tmp_path / "gen.jsonl").write_text("".join(json.dumps(r) + "\n" for r in [
        {"instance_id": "a", "source": "gen", "generator_class": "sm", "detected_class_petrivet": "sm", "size_bucket": "2-5", "places": 3},
        {"instance_id": "b", "source": "gen", "generator_class": "sm", "detected_class_petrivet": "mg", "size_bucket": "2-5", "places": 5},
        {"instance_id": "c", "source": "gen", "generator_class": "mg", "detected_class_petrivet": "mg", "size_bucket": "6-8", "places": 7},
    ]))
    (tmp_path / "fam.jsonl").write_text("".join(json.dumps(r) + "\n" for r in [{"instance_id": "c", "source": "fam", "places": 9}, {"instance_id": "d", "source": "fam", "places": 11}]))
    ctx = Stub(tmp_path, manifests=[tmp_path / "gen.jsonl", tmp_path / "fam.jsonl"])
    e0.run(ctx)
    n = ctx.numbers
    assert (n["e0.gen.n_manifest_lines"], n["e0.fam.n_manifest_lines"]) == (3, 2)
    assert (n[key("e0", "gen", "sm", "2-5", "n")], n[key("e0", "gen", "sm", "2-5", "n_same_class")], n[key("e0", "gen", "sm", "n")], n[key("e0", "gen", "mg", "6-8", "n")]) == (2, 1, 2, 1)
    assert (n["e0.n_unique_instances"], n["e0.n_duplicate_instances"]) == (4, 1)  # "c" is in both manifests


def test_e0_without_manifests_is_skipped(tmp_path):
    ctx = Stub(tmp_path)
    e0.run(ctx)
    assert ctx.skipped and not ctx.numbers


def test_e9_reports_the_baseline_run_of_every_solver(tmp_path):
    rows = [run_row("BASELINE", "lola", "TRUE", 25.0, "Q1"), run_row("BASELINE", "its-tools", "TRUE", 900.0, "Q1"), run_row("i1", "lola", "TRUE", 5.0)]
    ctx = Stub(tmp_path, runs=runs_frame(rows))
    e9.run(ctx)
    assert (ctx.numbers["e9.lola.baseline_wall_ms"], ctx.numbers["e9.its_tools.baseline_wall_ms"], ctx.numbers["e9.lola.query"]) == (25.0, 900.0, "Q1")


def test_e2b_before_and_after(tmp_path):
    meta = meta_frame([{"instance_id": i, "source": "mcc-small", "cls": "general", "family": None, "size": None, "places": 5.0, "transitions": 5.0} for i in ("i1", "i2", "i3")])
    general = [run_row("i1", "pv-general", "TRUE", 10.0), run_row("i2", "pv-general", "TRUE", 20.0), run_row("i3", "pv-general", "TRUE", 30.0)]
    before = [run_row("i1", "pv-dispatch", "TIMEOUT", 30003.0), run_row("i2", "pv-dispatch", "TRUE", 200.0), run_row("i3", "pv-dispatch", "TIMEOUT", 30003.0)]
    after = [run_row("i1", "pv-dispatch", "TRUE", 40.0), run_row("i2", "pv-dispatch", "TRUE", 40.0), run_row("i3", "pv-dispatch", "MEMOUT", 9000.0)]
    (tmp_path / "archive").mkdir()
    with gzip.open(tmp_path / e2b.ARCHIVE, "wt") as f:
        f.write("".join(json.dumps(r) + "\n" for r in before))
    ctx = Stub(tmp_path, runs=runs_frame(general + after), meta=meta)
    e2b.run(ctx)
    n = ctx.numbers
    p = key("e2b", "before", "mcc-small", "Q3") + "."
    assert n["e2b.before_present"] is True
    assert (n[p + "n_dispatch_answered"], n[p + "n_both"], n[p + "n_dispatch_fails_general_answers"]) == (1, 1, 2)
    assert n[p + "ratio_median"] == 10.0  # 200 ms against 20 ms
    q = key("e2b", "after", "mcc-small", "Q3") + "."
    assert (n[q + "n_dispatch_answered"], n[q + "n_both"], n[q + "n_dispatch_fails_general_answers"]) == (2, 2, 1)
    assert n[q + "ratio_median"] == statistics_median([4.0, 2.0])


def statistics_median(v):
    import statistics
    return statistics.median(v)


def test_e2b_without_the_archive_only_after(tmp_path):
    meta = meta_frame([{"instance_id": "i1", "source": "mcc-small", "cls": "general", "family": None, "size": None, "places": 5.0, "transitions": 5.0}])
    rows = [run_row("i1", "pv-general", "TRUE", 10.0), run_row("i1", "pv-dispatch", "TRUE", 20.0)]
    ctx = Stub(tmp_path, runs=runs_frame(rows), meta=meta)
    e2b.run(ctx)
    assert ctx.numbers["e2b.before_present"] is False
    assert key("e2b", "after", "mcc-small", "Q3", "n_both") in ctx.numbers and not any(k.startswith("e2b.before.") for k in ctx.numbers)
