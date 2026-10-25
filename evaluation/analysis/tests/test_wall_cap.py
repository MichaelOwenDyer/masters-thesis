import polars as pl

from analysis.e3b import wall_cap_cells


def test_only_runs_ended_by_the_wall_cap_are_listed_once_per_cell():
    rows = [
        {"source": "fam", "family": "efc_forkjoin", "instance_id": "i1", "query": "Q5", "tid": "", "timeout_ms": 10.0, "verdict": "TIMEOUT", "wall_ms": 30100.0},
        {"source": "fam", "family": "efc_forkjoin", "instance_id": "i1", "query": "Q5", "tid": "", "timeout_ms": 10.0, "verdict": "TIMEOUT", "wall_ms": 30300.0},
        {"source": "fam", "family": "efc_forkjoin", "instance_id": "i1", "query": "Q5", "tid": "", "timeout_ms": 100.0, "verdict": "TIMEOUT", "wall_ms": 30200.0},
        {"source": "mcc-small", "family": None, "instance_id": "i2", "query": "Q3", "tid": "", "timeout_ms": 10.0, "verdict": "UNKNOWN", "wall_ms": 45.0},
        {"source": "mcc-small", "family": None, "instance_id": "i3", "query": "Q3", "tid": "", "timeout_ms": 10.0, "verdict": "TIMEOUT", "wall_ms": 30001.0},
    ]
    out = wall_cap_cells(pl.DataFrame(rows))
    assert out.height == 3
    assert out.filter(pl.col("instance_id") == "i1").height == 2  # one row per cell (t = 10 and t = 100), repetitions merged
    assert out.filter(pl.col("instance_id") == "i3")["group"].to_list() == ["mcc-small"]  # the group falls back to the source
    assert out.filter((pl.col("instance_id") == "i1") & (pl.col("timeout_ms") == 10.0))["wall_ms"].to_list() == [30200.0]
