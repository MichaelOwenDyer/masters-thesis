import json
import statistics

import polars as pl

from analysis.common import series_max, series_min


def test_helpers_ignore_nulls_and_empty():
    s = pl.Series([3.5, None, 7.25, 1.0])
    assert (series_max(s), series_min(s)) == (7.25, 1.0)
    assert series_max(pl.Series([], dtype=pl.Float64)) is None


def test_e7_maxima_are_the_largest_cell_median_of_the_raw_runs(fake_dir, numbers):
    """E7 reports the largest median memory over repetitions per cell: recomputed here from the run log without polars."""
    cells: dict[tuple, list[float]] = {}
    for line in (fake_dir / "results" / "runs.jsonl").read_text().splitlines():
        r = json.loads(line)
        if r["campaign"] == "B" and r["instance_id"] != "BASELINE" and r.get("peak_rss_kib") is not None:
            cells.setdefault((r["solver_id"], r["instance_id"], r["query"], r.get("target_id")), []).append(r["peak_rss_kib"])
    expected: dict[str, float] = {}
    for (solver, *_), v in cells.items():
        m = statistics.median(v)
        expected[solver.replace("-", "_")] = max(expected.get(solver.replace("-", "_"), 0.0), m)
    got: dict[str, float] = {}
    for k, v in numbers.items():
        parts = k.split(".")
        if parts[0] == "e7" and parts[-1] == "rss_max_kib":
            got[parts[2]] = max(got.get(parts[2], 0.0), v)
    assert got and got == expected
