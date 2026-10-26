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


def test_memory_by_size_counts_memouts_and_missing_values(tmp_path):
    from analysis.e2 import _memory_by_size
    base = {"cls": "sm", "family": "sm_x", "size": 16.0}
    ok = pl.DataFrame([
        {**base, "rss_d": 8000.0, "rss_g": 8388608.0, "verdict_d": "TRUE", "verdict_g": "MEMOUT"},
        {**base, "rss_d": 9000.0, "rss_g": None, "verdict_d": "TRUE", "verdict_g": "TIMEOUT"},
    ])
    c = Stub(tmp_path)
    c.cfg = {"run": {"mem_limit_gib": 8}}
    _memory_by_size(c, ok, ["sm"])
    r = c.csvs["e2_memory_by_size"].row(0, named=True)
    assert r["n_g_memout"] == 1 and r["n_g_no_value"] == 1 and r["n_g_with_value"] == 1
    assert r["median_rss_g_mib"] == 8192.0 and abs(r["median_rss_d_mib"] - 8500 / 1024) < 1e-9
    assert (tmp_path / "e2_memory_by_size.svg").exists()
