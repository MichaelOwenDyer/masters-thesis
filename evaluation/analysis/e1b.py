"""E1b: cache coherence. The count comes from the testkit test `cache_coherence`, not from runs.jsonl.

Input (``--cache-coherence FILE``), either
  * a JSON object with ``differences`` (also accepted: ``n_differences``, ``count``) and optionally ``nets`` (``n_nets``,
    ``total``), or
  * a text file; the first line matching ``differences\\s*[:=]\\s*(\\d+)`` is used (``nets\\s*[:=]\\s*(\\d+)`` for the count).
Without the option the analysis is skipped.

Keys: e1b.n_differences, e1b.n_nets (when given), e1b.cp_upper_95 (when nets given and differences = 0).
"""

from __future__ import annotations

import json
import re

from .common import Ctx
from .stats import cp_upper


def parse(text: str) -> tuple[int | None, int | None]:
    try:
        v = json.loads(text)
        if isinstance(v, dict):
            d = next((v[k] for k in ("differences", "n_differences", "count") if k in v), None)
            n = next((v[k] for k in ("nets", "n_nets", "total") if k in v), None)
            return d, n
        if isinstance(v, int):
            return v, None
    except json.JSONDecodeError:
        pass
    d = re.search(r"differences\s*[:=]\s*(\d+)", text)
    n = re.search(r"nets\s*[:=]\s*(\d+)", text)
    if d is None and re.fullmatch(r"\s*\d+\s*", text):
        return int(text), None
    return (int(d.group(1)) if d else None), (int(n.group(1)) if n else None)


def run(ctx: Ctx) -> None:
    p = ctx.cache_coherence
    if p is None:
        ctx.skip("e1b", "no --cache-coherence file given")
        return
    if not p.is_file():
        ctx.skip("e1b", f"{p} not found")
        return
    d, n = parse(p.read_text())
    if d is None:
        ctx.skip("e1b", f"{p}: no difference count found")
        return
    ctx.num("e1b", "n_differences", d)
    if n is not None:
        ctx.num("e1b", "n_nets", n)
        if d == 0:
            ctx.num("e1b", "cp_upper_95", cp_upper(0, n))
