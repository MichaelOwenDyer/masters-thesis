import polars as pl

from analysis.common import RUN_SCHEMA, frame, use_memory_campaign


def row(campaign, inst, solver, verdict="TRUE", rss=143_000, rep=0, query="Q0"):
    return {"campaign": campaign, "instance_id": inst, "query": query, "tid": "", "solver_id": solver, "rep": rep,
            "verdict": verdict, "peak_rss_kib": rss, "run_id": f"{campaign}-{inst}-{solver}-{rep}"}


def build(rows):
    df = frame(rows, RUN_SCHEMA)
    return df.with_columns(pl.col("target_id").fill_null("").alias("tid"))


def test_memory_of_b_is_replaced_by_m():
    runs = build([
        row("B", "n1", "lola"), row("B", "n1", "lola", rep=1), row("B", "n2", "lola"), row("B", "n4", "lola", "MEMOUT", rss=8_388_000), row("B", "BASELINE", "lola", rss=29_000),
        row("A", "n1", "lola", rss=111),
        row("M", "n1", "lola", rss=5_000), row("M", "BASELINE", "lola", rss=3_000),
    ])
    out, info = use_memory_campaign(runs)
    b = out.filter(pl.col("campaign") == "B").sort("instance_id", "rep")
    got = {(r["instance_id"], r["rep"]): r["peak_rss_kib"] for r in b.iter_rows(named=True)}
    assert got == {("BASELINE", 0): 3_000, ("n1", 0): 5_000, ("n1", 1): 5_000, ("n2", 0): None, ("n4", 0): 8_388_000}
    assert out.filter(pl.col("campaign") == "A")["peak_rss_kib"].to_list() == [111]  # other campaigns untouched
    assert info["meta.memory_source"].startswith("campaign M")
    assert info["m.n_b_cells_without_m"] == 2 and info["m.n_verdict_differs"] == 0


def test_without_m_nothing_changes_and_the_source_says_so():
    runs = build([row("B", "n1", "lola")])
    out, info = use_memory_campaign(runs)
    assert out["peak_rss_kib"].to_list() == [143_000]
    assert "campaign B" in info["meta.memory_source"]


def test_verdict_differences_are_counted_and_limits_apart():
    runs = build([
        row("B", "n1", "lola", "TRUE"), row("M", "n1", "lola", "FALSE"),
        row("B", "n2", "lola", "TIMEOUT"), row("M", "n2", "lola", "TRUE"),
        row("B", "n3", "lola", "TRUE"), row("M", "n3", "lola", "TRUE"),
    ])
    _, info = use_memory_campaign(runs)
    assert info["m.n_verdict_differs"] == 2 and info["m.n_verdict_differs_not_limit"] == 1
