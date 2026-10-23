import math

import numpy as np
import pytest

from analysis.stats import cp_upper, hodges_lehmann, holm, wilcoxon_greater


def test_cp_zero_failures_matches_closed_form():
    # 1 - 0.05^(1/n); rule of three: about 3/n
    assert cp_upper(0, 100) == pytest.approx(1 - 0.05 ** 0.01)
    assert cp_upper(0, 1000) == pytest.approx(3 / 1000, rel=0.02)
    assert cp_upper(0, 0) is None
    assert cp_upper(5, 5) == 1.0


def test_cp_general_known_value():
    # one-sided 95 % upper bound for 1 failure in 10 trials (exact binomial): 0.3942
    assert cp_upper(1, 10) == pytest.approx(0.3942, abs=1e-3)


def test_holm_example():
    # classic example: p = .01 .04 .03 .005 with m = 4 -> adjusted .03 .06 .06 .02
    adj = holm([0.01, 0.04, 0.03, 0.005])
    assert adj == pytest.approx([0.03, 0.06, 0.06, 0.02])
    assert holm([None, 0.5, float("nan")]) == [None, 0.5, None]


def test_holm_monotone_and_capped():
    adj = holm([0.4, 0.5, 0.9])
    assert all(a <= 1 for a in adj) and adj == sorted(adj)


def test_hodges_lehmann():
    assert hodges_lehmann(np.array([1.0, 2.0, 3.0])) == pytest.approx(2.0)
    # robust to one outlier: Walsh averages of [1,2,3,100]
    assert hodges_lehmann(np.array([1.0, 2.0, 3.0, 100.0])) == pytest.approx(2.75)
    assert hodges_lehmann(np.array([])) is None


def test_wilcoxon_direction():
    d = np.arange(1, 21, dtype=float)
    _, p = wilcoxon_greater(d)
    assert p < 1e-3
    _, p2 = wilcoxon_greater(-d)
    assert p2 > 0.99
    assert wilcoxon_greater(np.zeros(5)) == (None, None)
