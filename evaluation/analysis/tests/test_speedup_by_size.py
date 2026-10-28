import polars as pl

from analysis.e2 import _by_size


class Stub:
    def __init__(self, tmp):
        self.numbers, self.csvs, self.tmp = {}, {}, tmp

    def num(self, *a):
        *parts, v = a
        self.numbers[".".join(parts)] = v

    def csv(self, name, df):
        self.csvs[name] = df

    def fig_path(self, name):
        return self.tmp / f"{name}.svg"


def row(size, td, tg, cd=False, cg=False, cls="sm"):
    return {"cls": cls, "family": "sm_x", "size": float(size), "t_d": td, "t_g": tg, "cens_d": cd, "cens_g": cg}


def test_speedup_summary_does_not_depend_on_the_sizes_tried(tmp_path):
    ok = pl.DataFrame([
        row(2, 10.0, 20.0), row(4, 10.0, 40.0),            # exact: speedups 2 and 4
        row(8, 10.0, 30000.0, cg=True),                    # plain exploration stopped: lower bound
        row(16, 30000.0, 30000.0, cd=True, cg=True),       # both stopped: no information
    ])
    c = Stub(tmp_path)
    _by_size(c, ok, ["sm"])
    d = c.csvs["e2_speedup_by_size"]
    assert d.filter(pl.col("size") == 8)["n_general_stopped"][0] == 1
    assert d.filter(pl.col("size") == 16)["median_speedup"][0] is None  # excluded, not counted as 1
    assert c.numbers["e2.family.sm_x.last_clean_size"] == 4 and c.numbers["e2.family.sm_x.last_clean_speedup"] == 4.0
    assert c.numbers["e2.family.sm_x.general_first_stop_size"] == 8 and c.numbers["e2.family.sm_x.dispatch_first_stop_size"] == 16
    assert (tmp_path / "e2_speedup_by_size.svg").exists()


def test_memory_runs_use_b_memory_for_timeouts_and_never_pool_questions(tmp_path):
    from analysis.e2 import _memory_by_size
    base = {"cls": "mg", "family": "mg_x", "size": 4.0, "source": "fam", "instance_id": "i", "tid": "", "rep": 0}
    runs = pl.DataFrame([
        {**base, "query": "Q1", "solver_id": "pv-general", "verdict": "TIMEOUT", "peak_rss_kib": None, "rss_b_kib": 8_000_000.0},
        {**base, "query": "Q2", "solver_id": "pv-general", "verdict": "FALSE", "peak_rss_kib": 7000.0, "rss_b_kib": 150_000.0},
        {**base, "query": "Q3", "solver_id": "pv-general", "verdict": "MEMOUT", "peak_rss_kib": 8_388_608.0, "rss_b_kib": 8_388_608.0},
        {**base, "query": "Q1", "solver_id": "pv-dispatch", "verdict": "TRUE", "peak_rss_kib": 34000.0, "rss_b_kib": 180_000.0},
    ])
    c = Stub(tmp_path)
    c.cfg = {"run": {"mem_limit_gib": 8}}
    c.runs_meta = lambda campaign=None: runs
    _memory_by_size(c, ["mg"])
    d = c.csvs["e2_memory_by_size"]
    row = lambda q, s: d.filter((pl.col("query") == q) & (pl.col("solver_id") == s)).row(0, named=True)
    assert d.height == 4  # one row per question and mode, no pooling
    assert row("Q1", "pv-general")["status"] == "time-out" and abs(row("Q1", "pv-general")["rss_mib"] - 8_000_000 / 1024) < 1e-9
    assert row("Q2", "pv-general")["status"] == "answered" and abs(row("Q2", "pv-general")["rss_mib"] - 7000 / 1024) < 1e-9  # M's value, not B's
    assert row("Q3", "pv-general")["status"] == "memory-out" and row("Q3", "pv-general")["rss_mib"] == 8192.0
    assert (tmp_path / "e2_memory_by_size.svg").exists()
