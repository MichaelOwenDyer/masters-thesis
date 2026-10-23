"""Small statistics helpers (scipy / numpy only)."""

from __future__ import annotations

import math

import numpy as np
from scipy import stats


def cp_upper(k: int, n: int, conf: float = 0.95) -> float | None:
    """One-sided Clopper-Pearson upper bound for k failures in n trials (exact; for k = 0 it is 1 - (1-conf)^(1/n))."""
    if n <= 0:
        return None
    if k >= n:
        return 1.0
    if k == 0:
        return 1.0 - (1.0 - conf) ** (1.0 / n)
    return float(stats.beta.ppf(conf, k + 1, n - k))


def holm(pvals: list[float | None]) -> list[float | None]:
    """Holm-Bonferroni adjusted p-values; ``None`` / NaN entries are ignored (stay ``None``)."""
    idx = [i for i, p in enumerate(pvals) if p is not None and not math.isnan(p)]
    order = sorted(idx, key=lambda i: pvals[i])
    m = len(order)
    out: list[float | None] = [None] * len(pvals)
    running = 0.0
    for rank, i in enumerate(order):
        adj = min(1.0, (m - rank) * pvals[i])
        running = max(running, adj)
        out[i] = running
    return out


def wilcoxon_greater(d: np.ndarray) -> tuple[float | None, float | None]:
    """Wilcoxon signed-rank test of H1: median(d) > 0. Zero differences are dropped. Returns (statistic, p)."""
    d = np.asarray(d, dtype=float)
    d = d[d != 0]
    if len(d) < 1:
        return None, None
    try:
        r = stats.wilcoxon(d, alternative="greater", zero_method="wilcox", method="auto")
    except ValueError:
        return None, None
    return float(r.statistic), float(r.pvalue)


def wilcoxon_two_sided(d: np.ndarray) -> tuple[float | None, float | None]:
    d = np.asarray(d, dtype=float)
    d = d[d != 0]
    if len(d) < 1:
        return None, None
    try:
        r = stats.wilcoxon(d, alternative="two-sided", zero_method="wilcox", method="auto")
    except ValueError:
        return None, None
    return float(r.statistic), float(r.pvalue)


def hodges_lehmann(d: np.ndarray) -> float | None:
    """Hodges-Lehmann estimate of the location shift: median of the Walsh averages (d_i + d_j)/2, i <= j."""
    d = np.asarray(d, dtype=float)
    if len(d) == 0:
        return None
    i, j = np.triu_indices(len(d))
    return float(np.median((d[i] + d[j]) / 2.0))


def quantiles(x, qs=(0.25, 0.5, 0.75, 0.95, 0.99)) -> dict[float, float | None]:
    x = np.asarray([v for v in x if v is not None and not math.isnan(v)], dtype=float)
    if len(x) == 0:
        return {q: None for q in qs}
    return {q: float(np.quantile(x, q)) for q in qs}


def median(x) -> float | None:
    x = [v for v in x if v is not None and not (isinstance(v, float) and math.isnan(v))]
    return float(np.median(x)) if x else None
