import csv
import json
import shutil
import xml.etree.ElementTree as ET

import polars as pl
import pytest

from analysis import fake
from analysis.common import adjudicate, load_runs, load_truth

from .conftest import run_pipeline


def test_outputs_exist(export):
    for f in ["numbers.json", "e1.csv", "e2_paired_tests.csv", "e3a.csv", "e3b_overshoot.csv", "e4.csv", "e5a.csv", "e5b.csv", "e6.csv", "e7_rss.csv"]:
        assert (export / f).is_file(), f
    svgs = sorted(p.name for p in (export / "figures").glob("*.svg"))
    assert {"e2_scaling.svg", "e2_paired_scatter.svg", "e3b_overshoot.svg", "e4_cactus_fam.svg", "e6_latency.svg", "e7_rss.svg"} <= set(svgs)
    for p in (export / "figures").glob("*.svg"):
        ET.parse(p)  # well-formed


def test_meta_and_no_skips(numbers):
    assert numbers["meta.generated_at"] == "T"
    for k in ("meta.eval_toml_sha256", "meta.runs_jsonl_sha256", "meta.truth_jsonl_sha256"):
        assert len(numbers[k]) == 64
    assert not [k for k in numbers if k.startswith("skipped.")]


def test_e1_zero_disagreements_and_bound(numbers):
    n = numbers["e1.mcc_small.n_conclusive"]
    assert n > 0 and numbers["e1.mcc_small.n_disagree"] == 0
    assert numbers["e1.mcc_small.cp_upper_95"] == pytest.approx(1 - 0.05 ** (1 / n))
    assert numbers["e1.gen.n_disagree"] == 0
    assert numbers["e1.n_truth_excluded"] > 0
    assert numbers["e1.adjudication_crosscheck.n_outcome_differs"] == 0
    assert numbers["e1.adjudication_crosscheck.n_only_in_runs"] == 0


def test_e1_counts_match_independent_count(fake_dir, numbers):
    """Count AGREE per source straight from the fake adjudication file (written independently by the generator)."""
    rows = [json.loads(l) for l in (fake_dir / "results" / "adjudication.jsonl").read_text().splitlines()]
    mine = sum(1 for r in rows if r["solver_id"] == "pv-dispatch" and r["source"] == "mcc-small" and r["outcome"] in ("AGREE", "DISAGREE"))
    assert numbers["e1.mcc_small.n_conclusive"] == mine


def test_numbers_keys_stable(numbers):
    # keys named in the SPEC example and the documented patterns
    for k in ["e1.mcc_small.n_conclusive", "e1b.n_differences", "e2.sm.hl_ms", "e2.sm.holm_p", "e3a.fam.share_correct",
              "e3b.t100.overshoot_median_ms", "e4.fam.pv_dispatch.par2_ms", "e5a.pv_dispatch.qr.yes.n_conclusive",
              "e5b.lola.qr.all.n_solved", "e6.pv_inproc_parsed.steady.p99_us", "e7.bytes_per_state.pv_dispatch.median"]:
        assert k in numbers, k
    assert all(k == k.lower() or k.startswith("skipped") for k in numbers)


def test_e2_hypothesis_directions(numbers):
    assert numbers["e2.sm.reject_h2"] is True and numbers["e2.sm.hl_ms"] > 0
    assert numbers["e2.general.median_ratio"] == pytest.approx(1.0, abs=0.2)  # dispatch == general on general nets
    assert numbers["e2.general.n_censored"] >= 0 and numbers["e2.sm.sens_n_pairs"] <= numbers["e2.sm.n_pairs"]


def test_e2_censoring_sensitivity(export, numbers):
    t = pl.read_csv(export / "e2_paired_tests.csv")
    assert (t["sens_n_pairs"] + t["n_censored"] == t["n_pairs"]).all()
    assert numbers["e2.alpha"] == 0.05


def test_e2_holm_is_across_classes(export):
    t = pl.read_csv(export / "e2_paired_tests.csv")
    assert (t["holm_p"] >= t["wilcoxon_p"] - 1e-12).all()


def test_e4_par2_and_cap(numbers):
    assert numbers["meta.cap_ms"] == 60000.0
    # unsolved cell counts 2 x cap in PAR-2
    n = numbers["e4.mcc_large.pv_general.n_cells"]
    s = numbers["e4.mcc_large.pv_general.n_solved"]
    assert s < n
    assert numbers["e4.mcc_large.pv_general.par2_ms"] >= (n - s) * 120000 / n - 1e-6


def test_e3b_threshold_from_config(numbers):
    assert numbers["e3b.threshold_ms"] == 50.0
    assert 0 <= numbers["e3b.t10.share_within_threshold"] <= 1


def test_e6_phases(numbers):
    assert numbers["e6.pv_inproc_parsed.process_first.n"] == 1
    assert "e6.pv_inproc_parsed.cold.n" not in numbers  # its only cold row is the process-first call
    assert numbers["e6.pv_inproc_cached.cold.n"] == 1  # cold first call of every other variant
    assert numbers["e6.pv_inproc_parsed.steady.n"] == 199
    assert numbers["e6.pv_inproc_cached.cold.p50_us"] > numbers["e6.pv_inproc_cached.steady.p50_us"]
    assert numbers["e6.baseline_noop.baseline.n"] == 200
    assert numbers["e6.n_dropped_no_answer"] == 1  # the aborted CRASH row
    assert numbers["e6.lola.steady.p50_us"] > numbers["e6.pv_inproc_parsed.steady.p50_us"]


def test_e6_fallback_to_campaign_d_runs(fake_dir, tmp_path):
    d = tmp_path / "fake"
    shutil.copytree(fake_dir, d)
    (d / "results" / "latency.jsonl").unlink()
    out = tmp_path / "e"
    assert run_pipeline(d, out) == 0
    n = json.loads((out / "numbers.json").read_text())
    assert "e6.subprocess_lola.steady.p50_us" in n and "e6.subprocess_lola.baseline.n" in n


def test_e7_bytes_per_state(numbers):
    assert numbers["e7.bytes_per_state.pv_dispatch.n"] == 30
    assert numbers["e7.bytes_per_state.pv_dispatch.median"] > 0


def test_deterministic(fake_dir, export, tmp_path):
    out2 = tmp_path / "again"
    assert run_pipeline(fake_dir, out2) == 0
    for f in ["numbers.json", "e2_paired_tests.csv", "e4.csv"]:
        assert (export / f).read_bytes() == (out2 / f).read_bytes()
    for f in (export / "figures").glob("*.svg"):
        assert f.read_bytes() == (out2 / "figures" / f.name).read_bytes(), f.name


def test_injected_disagreement_is_found(tmp_path):
    d = tmp_path / "fake"
    fake.generate(d, seed=5, inject_disagreements=3)
    out = tmp_path / "export"
    assert run_pipeline(d, out) == 0
    n = json.loads((out / "numbers.json").read_text())
    assert n["e1.n_disagree_total_subject"] == 3
    assert n["e1.gen.n_disagree"] == 3
    assert "e1.gen.cp_upper_95" in n and n["e1.gen.cp_upper_95"] is None  # bound only for zero disagreements
    # the generator's own adjudication.jsonl agrees (cross-check finds no differing outcome)
    assert n["e1.adjudication_crosscheck.n_outcome_differs"] == 0
    rows = list(csv.DictReader((out / "e1_disagreements.csv").open()))
    assert len(rows) == 3 and all(r["solver_id"] == "pv-dispatch" for r in rows)


def test_missing_optional_inputs_are_skipped(fake_dir, tmp_path):
    d = tmp_path / "fake"
    shutil.copytree(fake_dir, d)
    (d / "results" / "latency.jsonl").unlink()
    (d / "results" / "adjudication.jsonl").unlink()
    (d / "results" / "truth_excluded.jsonl").unlink()
    (d / "eval.toml").unlink()
    shutil.rmtree(d / "corpus")
    out = tmp_path / "export"
    from analysis.__main__ import main
    assert main(["--results", str(d / "results"), "--truth", str(d / "results" / "truth.jsonl"), "--out", str(out)]) == 0
    n = json.loads((out / "numbers.json").read_text())
    assert n["skipped.e1b"].startswith("no --cache-coherence")
    assert "skipped.e2" in n          # no manifest: no fam pairs with classes
    assert "e6.subprocess_lola.steady.p50_us" in n  # fallback: campaign D runs
    assert n["meta.eval_toml_sha256"] is None
    assert n["e1.mcc_small.n_conclusive"] > 0   # source recovered from truth corpus_source


def test_empty_results_do_not_crash(tmp_path):
    (tmp_path / "results").mkdir()
    (tmp_path / "results" / "runs.jsonl").write_text('{"run_id": "x", "verdict": "TRU')  # torn line only
    (tmp_path / "results" / "truth.jsonl").write_text("")
    from analysis.__main__ import main
    assert main(["--results", str(tmp_path / "results"), "--truth", str(tmp_path / "results" / "truth.jsonl"), "--out", str(tmp_path / "e")]) == 0
    n = json.loads((tmp_path / "e" / "numbers.json").read_text())
    assert n["meta.n_runs_unparsable_lines"] == 1
    assert {"skipped.e1", "skipped.e2", "skipped.e4", "skipped.e6", "skipped.e7"} <= set(n)


def test_out_of_scope_and_outcomes(tmp_path):
    """Unit test of the adjudication rules on a hand-built table."""
    run = lambda i, q, s, v, **kw: {"run_id": f"{i}{q}{s}", "campaign": "A", "instance_id": i, "query": q, "solver_id": s, "rep": 0, "verdict": v, **kw}
    runs = [run("a", "Q1", "pv-dispatch", "TRUE"), run("a", "Q2", "pv-dispatch", "FALSE"), run("b", "Q1", "pv-dispatch", "FALSE"),
            run("b", "Q3", "pv-dispatch", "UNKNOWN"), run("c", "Q0", "pv-dispatch", "COUNT", states=5, edges=7), run("c", "Q1", "pv-dispatch", "TRUE"),
            run("c", "Q1", "lola", "CRASH")]
    t = lambda i, q, v: {"instance_id": i, "query": q, "value": v, "source": "hand", "corpus_source": "gen"}
    truth = [t("a", "Q1", False), t("a", "Q2", False), t("b", "Q1", True), t("b", "Q3", True), t("c", "Q0", {"STATES": 5, "TRANSITIONS": 8})]
    rp, tp = tmp_path / "runs.jsonl", tmp_path / "truth.jsonl"
    rp.write_text("".join(json.dumps(r) + "\n" for r in runs))
    tp.write_text("".join(json.dumps(r) + "\n" for r in truth))
    adj = adjudicate(load_runs(rp)[0], load_truth(tp))
    got = {(r["instance_id"], r["query"], r["solver_id"]): r["outcome"] for r in adj.iter_rows(named=True)}
    assert got[("a", "Q1", "pv-dispatch")] == "OUT_OF_SCOPE"   # Q2 is false on a
    assert got[("a", "Q2", "pv-dispatch")] == "AGREE"
    assert got[("b", "Q1", "pv-dispatch")] == "DISAGREE"
    assert got[("b", "Q3", "pv-dispatch")] == "UNKNOWN"
    assert got[("c", "Q0", "pv-dispatch")] == "DISAGREE"       # edges differ
    assert got[("c", "Q1", "pv-dispatch")] == "NO_TRUTH"
    assert got[("c", "Q1", "lola")] == "CRASH"
