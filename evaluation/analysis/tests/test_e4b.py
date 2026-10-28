import polars as pl

from analysis import e4b


class Stub:
    def __init__(self, tmp):
        self.tmp, self.tables, self.cfg, self.skipped = tmp, {}, {"run": {"mem_limit_gib": 8}}, []

    def csv(self, name, df):
        self.tables[name] = df

    def fig_path(self, name):
        return self.tmp / f"{name}.svg"

    def skip(self, name, why):
        self.skipped.append(name)


def cell(src, inst, q, solver, verdict, wall, rss=None, rss_b=None, fam=None, size=None):
    return {"source": src, "instance_id": inst, "query": q, "tid": "", "solver_id": solver, "verdict": verdict, "wall_ms": wall,
            "solved": verdict in ("TRUE", "FALSE", "COUNT"), "rss_kib": rss, "rss_b_kib": rss_b, "family": fam, "size": size, "cls": "sm"}


def by_cell(rows):
    out = {}
    for r in rows:
        out.setdefault((r["source"], r["instance_id"], r["query"], r["tid"]), {})[r["solver_id"]] = r
    return out


def test_head_to_head_categories_sum_to_the_cells_and_use_the_cap(tmp_path):
    rows = []
    # petrivet 100 ms vs LoLA 400 ms (faster), 100 vs 100 (equal), 400 vs 100 (slower), petrivet time-out vs LoLA 50, both time out, LoLA time-out
    spec = [("a", "TRUE", 100, "TRUE", 400), ("b", "TRUE", 100, "TRUE", 100), ("c", "TRUE", 400, "TRUE", 100),
            ("d", "TIMEOUT", 30000, "TRUE", 50), ("e", "TIMEOUT", 30000, "TIMEOUT", 30000), ("f", "TRUE", 80, "TIMEOUT", 30000)]
    for inst, v1, t1, v2, t2 in spec:
        rows.append(cell("mcc-small", inst, "Q1", "pv-dispatch", v1, t1, rss=7000.0))
        rows.append(cell("mcc-small", inst, "Q1", "lola", v2, t2, rss=7000.0, rss_b=9_000_000.0))
    c = Stub(tmp_path)
    e4b._mcc(c, by_cell(rows), 30000.0)
    r = c.tables["e4b_time_headtohead"].filter((pl.col("reference") == "lola")).row(0, named=True)
    assert (r["n_cells"], r["n_both"], r["n_faster2"], r["n_equal2"], r["n_slower2"]) == (6, 3, 1, 1, 1)
    assert (r["n_only_reference"], r["n_only_petrivet"], r["n_neither"]) == (1, 1, 1)
    assert r["n_cells"] == r["n_both"] + r["n_only_reference"] + r["n_only_petrivet"] + r["n_neither"]
    assert (tmp_path / "e4b_time_scatter_mcc_small.svg").exists() and (tmp_path / "e4b_memory_scatter_mcc_small.svg").exists()
    m = c.tables["e4b_memory_headtohead"].filter(pl.col("reference") == "lola").row(0, named=True)
    assert m["n_with_timeout_value"] == 1  # LoLA's time-out in cell f has B's memory value


def test_frontier_needs_every_question_answered(tmp_path):
    rows = []
    for size, ok in ((2, True), (4, True), (8, False)):
        for q in ("Q1", "Q2", "Q3", "Q4"):
            v = "TRUE" if ok or q != "Q3" else "TIMEOUT"
            rows.append(cell("fam", f"i{size}", q, "lola", v, 20.0 if v == "TRUE" else 30000.0, rss=7000.0, fam="circuit", size=float(size)))
    c = Stub(tmp_path)
    e4b._fam(c, pl.DataFrame(rows, infer_schema_length=None), 30000.0)
    f = c.tables["e4b_frontier"].row(0, named=True)
    assert (f["max_size_all_solved"], f["max_size_attempted"]) == (4, 8)  # one question open at size 8
    s = c.tables["e4b_scaling_fam"].filter(pl.col("size") == 8).row(0, named=True)
    assert s["n_failed"] == 1 and s["total_ms"] == 3 * 20.0 + 30000.0


def test_par2_does_not_charge_an_unknown_before_the_cap():
    from analysis.e4 import summarise
    cap = 30000.0
    rows = [
        {"verdict": "TRUE", "wall_ms": 100.0, "solved": True, "outcome": "AGREE"},
        {"verdict": "UNKNOWN", "wall_ms": 25.0, "solved": False, "outcome": None},      # declined early: costs its 25 ms
        {"verdict": "TIMEOUT", "wall_ms": 30100.0, "solved": False, "outcome": None},   # costs 2 x cap
        {"verdict": "UNKNOWN", "wall_ms": 30000.0, "solved": False, "outcome": None},   # unknown at the cap: costs 2 x cap
    ]
    d = summarise(pl.DataFrame(rows, infer_schema_length=None), [], 5, cap)  # 5 cells: one has no record at all
    assert d["par2_ms"] == (100.0 + 25.0 + 3 * 2 * cap) / 5
    assert (d["n_solved"], d["n_unknown"], d["n_timeout"], d["n_missing"]) == (1, 2, 1, 1)
