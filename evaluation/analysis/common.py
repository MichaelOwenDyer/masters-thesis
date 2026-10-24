"""Loading, the shared context, numbers.json collection and plotting helpers.

Input schemas (all JSONL, one object per line; a torn or unparsable line is skipped and counted):

``results/runs.jsonl`` (petrivet-eval/src/record.rs, ``RunRecord``): run_id, campaign (A|B|C|D|TRUTH), instance_id,
query, target_id?, solver_id, rep, verdict (TRUE FALSE COUNT UNKNOWN TIMEOUT MEMOUT NOT_SUPPORTED INPUT_REJECTED CRASH
RUNNER_ERROR SHARD_FAILED), wall_ms, cpu_user_ms, cpu_sys_ms, peak_rss_kib?, parse_ms?, analyze_ms?, timeout_ms?,
limit_ms, mem_limit_kib, exit_code?, signal?, states?, edges?, convert_ms?, env_id, verdict_differs, detail?, ...
The startup baseline runs have ``instance_id == "BASELINE"``; labelling runs of the truth stage have campaign ``TRUTH``.

``results/truth.jsonl`` (truth.rs, ``TruthLine``): instance_id, query, target_id?, value (bool, or {"STATES":n,
"TRANSITIONS":m} for Q0), source (mcc-oracle | construction | hand | lola+tina ...), corpus_source, tools.

``results/adjudication.jsonl`` (adjudicate.rs, ``Line``): run_id, campaign, instance_id, query, target_id?, solver_id,
outcome (AGREE DISAGREE UNKNOWN TIMEOUT MEMOUT INPUT_REJECTED CRASH RUNNER_ERROR NOT_SUPPORTED NO_TRUTH OUT_OF_SCOPE),
truth_value?, truth_source?, detail?, answer?, source?, pattern?, verdict_differs.

Manifest lines (instance.rs, ``Instance``): instance_id, path, source, places, transitions, arcs,
generator_class?, detected_class_petrivet?, family?, size_param?, mcc_family?, ...
"""

from __future__ import annotations

import datetime as _dt
import hashlib
import json
import math
import re
import traceback
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

import numpy as np
import polars as pl

DEFINITE = ("TRUE", "FALSE", "COUNT")
SUBJECT = "pv-dispatch"

RUN_SCHEMA: dict[str, Any] = {
    "run_id": pl.Utf8, "campaign": pl.Utf8, "instance_id": pl.Utf8, "query": pl.Utf8, "target_id": pl.Utf8,
    "solver_id": pl.Utf8, "rep": pl.Int64, "verdict": pl.Utf8, "wall_ms": pl.Float64, "cpu_user_ms": pl.Float64,
    "cpu_sys_ms": pl.Float64, "peak_rss_kib": pl.Float64, "parse_ms": pl.Float64, "analyze_ms": pl.Float64,
    "timeout_ms": pl.Float64, "limit_ms": pl.Float64, "mem_limit_kib": pl.Float64, "states": pl.Float64,
    "edges": pl.Float64, "convert_ms": pl.Float64, "env_id": pl.Utf8, "verdict_differs": pl.Boolean,
    "detail": pl.Utf8, "started_unix_ms": pl.Float64,
}


# ---------------------------------------------------------------------------------------------------------
# generic helpers
# ---------------------------------------------------------------------------------------------------------

def key(*parts: Any) -> str:
    """Stable numbers.json key: parts lowercased, non-alphanumerics replaced by '_', joined with '.'."""
    return ".".join(re.sub(r"[^a-z0-9]+", "_", str(p).strip().lower()).strip("_") or "none" for p in parts)


def clean(v: Any) -> Any:
    if isinstance(v, (np.floating, float)):
        f = float(v)
        return None if math.isnan(f) or math.isinf(f) else f
    if isinstance(v, np.integer):
        return int(v)
    if isinstance(v, np.bool_):
        return bool(v)
    return v


def sha256_file(path: Path | None) -> str | None:
    if path is None or not path.is_file():
        return None
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_jsonl(path: Path) -> tuple[list[dict], int]:
    """All parsable objects of a JSONL file and the number of unparsable non-empty lines."""
    rows: list[dict] = []
    bad = 0
    with path.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                v = json.loads(line)
            except json.JSONDecodeError:
                bad += 1
                continue
            if isinstance(v, dict):
                rows.append(v)
            else:
                bad += 1
    return rows, bad


def frame(rows: list[dict], schema: dict[str, Any]) -> pl.DataFrame:
    """DataFrame with exactly the schema's columns; absent keys become null, extra keys are ignored."""
    data = {c: [r.get(c) for r in rows] for c in schema}
    return pl.DataFrame(data, schema=schema, strict=False)


def load_runs(path: Path) -> tuple[pl.DataFrame, int]:
    rows, bad = read_jsonl(path)
    for r in rows:
        if r.get("verdict_differs") is None:
            r["verdict_differs"] = False
    df = frame(rows, RUN_SCHEMA)
    return df.with_columns(pl.col("target_id").fill_null("").alias("tid")), bad


MEMORY_KEYS = ["instance_id", "query", "tid", "solver_id"]


def use_memory_campaign(runs: pl.DataFrame) -> tuple[pl.DataFrame, dict[str, Any]]:
    """Memory comes from campaign M, not from B.

    The peak memory in B's records is what ``wait4`` reports for the child of the runner, and that contains the
    runner's own footprint (a floor of about 143 MiB for every tool). Campaign M repeats every cell of B once with the
    tool started through the ``rss`` wrapper (petrivet-eval/src/measure.rs). When M records exist, ``peak_rss_kib`` of
    every B record is replaced by M's value for the same (instance, query, target, solver), or, when M has none, by null
    (a run killed at the wall cap leaves no value) except for a memory-out, which keeps B's value (the limit). Without M records B's values stay and the numbers say so.
    The verdicts of M and B are compared as a sanity check of the wrapper (it must not change an answer)."""
    info: dict[str, Any] = {}
    m = runs.filter(pl.col("campaign") == "M")
    if m.height == 0:
        info["meta.memory_source"] = "campaign B (wait4 includes the runner's own footprint: small values are not reliable)"
        return runs, info
    mem = m.filter(pl.col("rep") == 0).select(MEMORY_KEYS + [pl.col("peak_rss_kib").alias("_rss_m"), pl.col("verdict").alias("_verdict_m")]).unique(subset=MEMORY_KEYS, keep="first")
    is_b = pl.col("campaign") == "B"
    joined = runs.join(mem, on=MEMORY_KEYS, how="left")
    # B records without an M value: a memory-out keeps B's value (the limit, far above the runner's footprint); the
    # rest has none (time-outs are killed together with the wrapper)
    kept_b = pl.when(pl.col("verdict") == "MEMOUT").then(pl.col("peak_rss_kib"))
    out = joined.with_columns(pl.when(is_b).then(pl.coalesce(pl.col("_rss_m"), kept_b)).otherwise(pl.col("peak_rss_kib")).alias("peak_rss_kib"))
    b0 = joined.filter(is_b & (pl.col("rep") == 0) & pl.col("_verdict_m").is_not_null())
    info["meta.memory_source"] = "campaign M (runs started through the rss wrapper)"
    info["m.n_runs"] = m.height
    info["m.n_cells_matched_in_b"] = b0.height
    info["m.n_b_cells_without_m"] = joined.filter(is_b & (pl.col("rep") == 0) & pl.col("_verdict_m").is_null() & (pl.col("instance_id") != "BASELINE")).height
    info["m.n_with_memory_value"] = m.filter(pl.col("peak_rss_kib").is_not_null()).height
    # a time-out or memory-out near the cap can legitimately flip between two runs: those are counted apart
    near = ["TIMEOUT", "MEMOUT"]
    differs = b0.filter(pl.col("verdict") != pl.col("_verdict_m"))
    info["m.n_verdict_differs"] = differs.height
    info["m.n_verdict_differs_not_limit"] = differs.filter(~pl.col("verdict").is_in(near) & ~pl.col("_verdict_m").is_in(near)).height
    return out.drop(["_rss_m", "_verdict_m"]), info


def load_truth(path: Path) -> pl.DataFrame:
    rows, _ = read_jsonl(path)
    out = []
    for r in rows:
        v = r.get("value")
        obj = v if isinstance(v, dict) else {}
        out.append({
            "instance_id": r.get("instance_id"), "query": r.get("query"), "tid": r.get("target_id") or "",
            "truth_bool": v if isinstance(v, bool) else None,
            "truth_states": obj.get("STATES"), "truth_edges": obj.get("TRANSITIONS"),
            "truth_kind": "bool" if isinstance(v, bool) else ("count" if isinstance(v, dict) else "other"),
            "truth_source": r.get("source"), "truth_corpus_source": r.get("corpus_source"),
        })
    schema = {"instance_id": pl.Utf8, "query": pl.Utf8, "tid": pl.Utf8, "truth_bool": pl.Boolean,
              "truth_states": pl.Float64, "truth_edges": pl.Float64, "truth_kind": pl.Utf8,
              "truth_source": pl.Utf8, "truth_corpus_source": pl.Utf8}
    return frame(out, schema)


def _num(v: Any) -> float | None:
    if isinstance(v, bool):
        return None
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, str):
        m = re.search(r"-?\d+(\.\d+)?", v)
        return float(m.group(0)) if m else None
    return None


def load_meta(paths: list[Path], truth: pl.DataFrame, adj_rows: list[dict] | None) -> pl.DataFrame:
    """One row per instance: source, class, family, size, places, transitions. Source falls back to the truth's
    ``corpus_source`` and the adjudication's ``source`` when no manifest line exists."""
    meta: dict[str, dict] = {}
    for p in paths:
        if not p.is_file():
            continue
        rows, _ = read_jsonl(p)
        for r in rows:
            iid = r.get("instance_id")
            if not iid:
                continue
            cls = r.get("detected_class_petrivet") or r.get("generator_class") or r.get("detected_class_testkit")
            fam = r.get("family") or r.get("mcc_family")
            meta[iid] = {"instance_id": iid, "source": r.get("source"), "cls": (str(cls).lower() if cls else None),
                         "family": fam, "size": _num(r.get("size_param")), "places": _num(r.get("places")),
                         "transitions": _num(r.get("transitions"))}
    for r in truth.select("instance_id", "truth_corpus_source").unique().iter_rows(named=True):
        m = meta.setdefault(r["instance_id"], {"instance_id": r["instance_id"]})
        if not m.get("source") and r["truth_corpus_source"]:
            m["source"] = r["truth_corpus_source"]
    for r in adj_rows or []:
        iid = r.get("instance_id")
        if iid and r.get("source"):
            m = meta.setdefault(iid, {"instance_id": iid})
            if not m.get("source"):
                m["source"] = r["source"]
    schema = {"instance_id": pl.Utf8, "source": pl.Utf8, "cls": pl.Utf8, "family": pl.Utf8, "size": pl.Float64,
              "places": pl.Float64, "transitions": pl.Float64}
    return frame(list(meta.values()), schema)


def with_meta(df: pl.DataFrame, meta: pl.DataFrame) -> pl.DataFrame:
    out = df.join(meta, on="instance_id", how="left")
    return out.with_columns(pl.col("source").fill_null("unknown"), pl.col("cls").fill_null("unknown"))


# ---------------------------------------------------------------------------------------------------------
# adjudication (independent re-implementation of adjudicate.rs rules, so --truth drives the analysis)
# ---------------------------------------------------------------------------------------------------------

def classify(row: dict, oos: bool) -> str:
    v = row["verdict"]
    if v == "CRASH":
        return "CRASH"
    if oos:
        return "OUT_OF_SCOPE"
    if v in ("TRUE", "FALSE"):
        if row["truth_kind"] is None:
            return "NO_TRUTH"
        if row["truth_kind"] != "bool":
            return "RUNNER_ERROR"
        return "AGREE" if (v == "TRUE") == row["truth_bool"] else "DISAGREE"
    if v == "COUNT":
        if row["states"] is None or row["edges"] is None:
            return "RUNNER_ERROR"
        if row["truth_kind"] is None:
            return "NO_TRUTH"
        if row["truth_kind"] != "count":
            return "RUNNER_ERROR"
        ok = (row["truth_states"] is None or row["truth_states"] == row["states"]) and \
             (row["truth_edges"] is None or row["truth_edges"] == row["edges"])
        return "AGREE" if ok else "DISAGREE"
    if v in ("UNKNOWN", "NOT_SUPPORTED", "TIMEOUT", "MEMOUT", "INPUT_REJECTED", "RUNNER_ERROR", "SHARD_FAILED"):
        return v
    return "RUNNER_ERROR"


def adjudicate(runs: pl.DataFrame, truth: pl.DataFrame, campaigns=("A", "B")) -> pl.DataFrame:
    """Repetition 0 of every (campaign cell, solver) joined with truth; column ``outcome`` as in adjudicate.rs.
    A cell that appears in two campaigns is counted once (first campaign in alphabetical order)."""
    r = runs.filter((pl.col("instance_id") != "BASELINE") & pl.col("campaign").is_in(list(campaigns)) & (pl.col("rep") == 0))
    r = r.sort("campaign").unique(subset=["instance_id", "query", "tid", "solver_id"], keep="first", maintain_order=True)
    r = r.join(truth, on=["instance_id", "query", "tid"], how="left")
    q2_false = set(truth.filter((pl.col("query") == "Q2") & (pl.col("truth_bool") == False)).get_column("instance_id").to_list())  # noqa: E712
    outcomes = []
    for row in r.iter_rows(named=True):
        oos = row["query"] in ("Q1", "Q3", "Q4") and row["instance_id"] in q2_false
        outcomes.append(classify(row, oos))
    return r.with_columns(pl.Series("outcome", outcomes, dtype=pl.Utf8))


# ---------------------------------------------------------------------------------------------------------
# context
# ---------------------------------------------------------------------------------------------------------

@dataclass
class Ctx:
    results: Path
    truth_path: Path
    out: Path
    runs: pl.DataFrame
    truth: pl.DataFrame
    meta: pl.DataFrame
    cfg: dict
    subject: str = SUBJECT
    adj_rows: list[dict] | None = None
    cache_coherence: Path | None = None
    latency_path: Path | None = None
    truth_excluded: list[dict] = field(default_factory=list)
    overshoot_threshold_ms: float | None = None
    cap_ms: float | None = None
    numbers: dict[str, Any] = field(default_factory=dict)
    written: list[str] = field(default_factory=list)
    _adj: pl.DataFrame | None = None

    # --- numbers ---
    def num(self, *parts_and_value: Any) -> None:
        *parts, value = parts_and_value
        self.numbers[key(*parts)] = clean(value)

    def skip(self, exp: str, reason: str) -> None:
        self.numbers["skipped." + key(exp)] = reason

    # --- derived data ---
    @property
    def adj(self) -> pl.DataFrame:
        """Adjudicated rows (A and B campaigns) with source and class attached."""
        if self._adj is None:
            self._adj = with_meta(adjudicate(self.runs, self.truth), self.meta)
        return self._adj

    def runs_meta(self, campaign: str | tuple[str, ...] | None = None) -> pl.DataFrame:
        r = self.runs.filter(pl.col("instance_id") != "BASELINE")
        if campaign is not None:
            cs = [campaign] if isinstance(campaign, str) else list(campaign)
            r = r.filter(pl.col("campaign").is_in(cs))
        return with_meta(r, self.meta)

    def cap(self) -> float | None:
        """Common wall cap in ms for PAR-2 and censoring: eval.toml [run].wall_cap_secs, else the largest limit_ms of B."""
        if self.cap_ms is not None:
            return self.cap_ms
        c = self.cfg.get("run", {}).get("wall_cap_secs")
        if c:
            return float(c) * 1000.0
        b = self.runs.filter(pl.col("campaign") == "B")["limit_ms"].drop_nulls()
        return float(b.max()) if len(b) else None

    # --- outputs ---
    def csv(self, name: str, df: pl.DataFrame) -> None:
        self.out.mkdir(parents=True, exist_ok=True)
        df.write_csv(self.out / f"{name}.csv", float_precision=6)
        self.written.append(f"{name}.csv")

    def fig_path(self, name: str) -> Path:
        d = self.out / "figures"
        d.mkdir(parents=True, exist_ok=True)
        self.written.append(f"figures/{name}.svg")
        return d / f"{name}.svg"


# ---------------------------------------------------------------------------------------------------------
# plotting (matplotlib, deterministic SVG)
# ---------------------------------------------------------------------------------------------------------

# Okabe-Ito, colour-blind safe
PALETTE = ["#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9", "#000000", "#999999"]


def plt():
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as _plt
    _plt.rcParams.update({
        "svg.hashsalt": "petrivet", "svg.fonttype": "none", "font.size": 9, "axes.spines.top": False,
        "axes.spines.right": False, "axes.grid": True, "grid.alpha": 0.25, "figure.dpi": 100,
    })
    return _plt


def save_svg(fig, path: Path) -> None:
    fig.savefig(path, format="svg", metadata={"Date": None, "Creator": "petrivet-analysis"}, bbox_inches="tight")
    plt().close(fig)


def color_map(names) -> dict[str, str]:
    return {n: PALETTE[i % len(PALETTE)] for i, n in enumerate(sorted(set(names)))}


# Names used in titles, axes and legends: plain language, no internal ids. The CSVs and numbers.json keep the ids.
SOLVER_LABEL = {
    "pv-dispatch": "petrivet: automatic choice of method",
    "pv-general": "petrivet: plain state-space exploration",
    "pv-poly": "petrivet: fast methods only",
    "lola": "LoLA",
    "tina": "TINA",
}
# one colour per solver in every chart
SOLVER_COLOR = {"pv-dispatch": "#0072B2", "pv-general": "#D55E00", "pv-poly": "#CC79A7", "lola": "#009E73", "tina": "#E69F00"}
# structural classes, narrowest first
CLASS_ORDER = ["circuit", "sm", "mg", "efc", "ac", "general"]
CLASS_LABEL = {
    "circuit": "circuit",
    "sm": "state machine",
    "mg": "marked graph",
    "efc": "extended free-choice",
    "ac": "asymmetric choice",
    "general": "general (no special structure)",
}
SOURCE_LABEL = {
    "gen": "generated nets",
    "fam": "generated net families",
    "calibration": "hand-built calibration nets",
    "mcc_small": "benchmark models, small (MCC)",
    "mcc_large": "benchmark models, large (MCC)",
}
FAMILY_LABEL = {
    "circuit": "plain ring",
    "sm_hub_and_spokes": "hub and spokes",
    "sm_two_cycles": "two cycles, one-way switch",
    "sm_two_cycles_n_tokens": "two cycles, n tokens",
    "sm_choice_ring": "ring of choices",
    "sm_ring_with_sink": "ring with a token sink",
    "mg_buffer_ring": "ring of buffers",
    "mg_buffer_ring_unmarked_circuit": "ring of buffers, token-free circuit",
    "mg_forkjoin_cycle": "fork and join, cyclic",
    "mg_forkjoin_unbounded": "fork and join, unbounded",
    "efc_forkjoin": "fork and join with choice",
    "efc_forkjoin_leaky": "fork and join with choice, leaky",
    "ac_cycle": "cycle with choices",
    "ac_cycle_no_low_token": "cycle with choices, no start token",
    "general_cycle": "cycle",
}


# short names for tick labels
SOLVER_SHORT = {
    "pv-dispatch": "petrivet\nautomatic",
    "pv-general": "petrivet\nexploration",
    "pv-poly": "petrivet\nfast only",
    "lola": "LoLA",
    "tina": "TINA",
}


def solver_label(s: str) -> str:
    return SOLVER_LABEL.get(s, s)


def source_label(s: str) -> str:
    return SOURCE_LABEL.get(s) or SOURCE_LABEL.get(s.replace("-", "_"), s)


# ---------------------------------------------------------------------------------------------------------
# running modules
# ---------------------------------------------------------------------------------------------------------

def run_module(ctx: Ctx, exp: str, fn: Callable[[Ctx], None]) -> bool:
    try:
        fn(ctx)
        return True
    except Exception as e:  # a bug in one analysis must not hide the others
        traceback.print_exc()
        ctx.numbers["skipped." + key(exp)] = f"error: {type(e).__name__}: {e}"
        return False


def utc_now() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
