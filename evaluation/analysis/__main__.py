"""python -m analysis --results DIR --truth FILE --out export/   (run from the evaluation/ directory)"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

import polars as pl

from . import __version__, e1, e1b, e2, e3a, e3b, e4, e5a, e5b, e6, e7
from .common import Ctx, load_meta, load_runs, load_truth, read_jsonl, run_module, sha256_file, utc_now

MODULES = [("e1", e1.run), ("e1b", e1b.run), ("e2", e2.run), ("e3a", e3a.run), ("e3b", e3b.run), ("e4", e4.run),
           ("e5a", e5a.run), ("e5b", e5b.run), ("e6", e6.run), ("e7", e7.run)]


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="python -m analysis", description=__doc__)
    ap.add_argument("--results", type=Path, required=True, help="results directory (runs.jsonl, optionally adjudication.jsonl, latency.jsonl, truth_excluded.jsonl)")
    ap.add_argument("--truth", type=Path, required=True, help="results/truth.jsonl")
    ap.add_argument("--out", type=Path, required=True, help="export directory (numbers.json, *.csv, figures/*.svg)")
    ap.add_argument("--config", type=Path, help="eval.toml (default: <results>/../eval.toml)")
    ap.add_argument("--manifest", type=Path, action="append", default=[], help="corpus manifest JSONL (repeatable; default: corpus/manifest.jsonl and corpus/mcc/manifest.jsonl next to results/)")
    ap.add_argument("--adjudication", type=Path, help="adjudication.jsonl (default: <results>/adjudication.jsonl; used for patterns and a cross-check)")
    ap.add_argument("--latency", type=Path, help="latency.jsonl (default: <results>/latency.jsonl)")
    ap.add_argument("--cache-coherence", type=Path, help="file with the cache_coherence difference count (E1b); skipped when absent")
    ap.add_argument("--subject", default="pv-dispatch")
    ap.add_argument("--cap-ms", type=float, help="common wall cap for PAR-2 / censoring (default eval.toml [run].wall_cap_secs)")
    ap.add_argument("--overshoot-threshold-ms", type=float, help="H3b threshold (default eval.toml [e3].overshoot_threshold_ms)")
    ap.add_argument("--only", help="comma separated experiment ids (default all)")
    ap.add_argument("--generated-at", help="override the timestamp (reproducible output in tests)")
    a = ap.parse_args(argv)

    runs_path = a.results / "runs.jsonl"
    if not runs_path.is_file():
        print(f"error: {runs_path} not found", file=sys.stderr)
        return 1
    if not a.truth.is_file():
        print(f"error: {a.truth} not found", file=sys.stderr)
        return 1
    runs, bad = load_runs(runs_path)
    truth = load_truth(a.truth)

    cfg_path = a.config or a.results.resolve().parent / "eval.toml"
    cfg = tomllib.loads(cfg_path.read_text()) if cfg_path.is_file() else {}
    adj_path = a.adjudication or a.results / "adjudication.jsonl"
    adj_rows = read_jsonl(adj_path)[0] if adj_path.is_file() else None
    base = a.results.resolve().parent
    manifests = a.manifest or [base / "corpus" / "manifest.jsonl", base / "corpus" / "mcc" / "manifest.jsonl"]
    meta = load_meta(manifests, truth, adj_rows)
    excl_path = a.results / "truth_excluded.jsonl"
    excluded = read_jsonl(excl_path)[0] if excl_path.is_file() else []

    ctx = Ctx(results=a.results, truth_path=a.truth, out=a.out, runs=runs, truth=truth, meta=meta, cfg=cfg, subject=a.subject,
              adj_rows=adj_rows, cache_coherence=a.cache_coherence, latency_path=a.latency or a.results / "latency.jsonl",
              truth_excluded=excluded, overshoot_threshold_ms=a.overshoot_threshold_ms, cap_ms=a.cap_ms)
    a.out.mkdir(parents=True, exist_ok=True)

    ctx.numbers["meta.generated_at"] = a.generated_at or utc_now()
    ctx.numbers["meta.eval_toml_sha256"] = sha256_file(cfg_path)
    ctx.numbers["meta.runs_jsonl_sha256"] = sha256_file(runs_path)
    ctx.numbers["meta.truth_jsonl_sha256"] = sha256_file(a.truth)
    ctx.numbers["meta.adjudication_jsonl_sha256"] = sha256_file(adj_path) if adj_rows is not None else None
    ctx.numbers["meta.analysis_version"] = __version__
    ctx.numbers["meta.n_runs"] = runs.height
    ctx.numbers["meta.n_runs_unparsable_lines"] = bad
    ctx.numbers["meta.n_truth_lines"] = truth.height
    ctx.numbers["meta.n_manifest_instances"] = meta.filter(pl.col("cls").is_not_null()).height
    ctx.numbers["meta.cap_ms"] = ctx.cap()
    if not any(Path(m).is_file() for m in manifests):
        ctx.numbers["meta.manifest_note"] = "no manifest found: classes unknown, family/size unavailable (E2 needs them)"

    only = set(a.only.split(",")) if a.only else None
    ok = True
    for exp, fn in MODULES:
        if only and exp not in only:
            continue
        ok &= run_module(ctx, exp, fn)

    ctx.numbers = {k: v for k, v in sorted(ctx.numbers.items())}
    (a.out / "numbers.json").write_text(json.dumps(ctx.numbers, indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    skipped = [k for k in ctx.numbers if k.startswith("skipped.")]
    print(f"wrote {a.out / 'numbers.json'} ({len(ctx.numbers)} keys), {len([w for w in ctx.written if w.endswith('.csv')])} csv, "
          f"{len([w for w in ctx.written if w.endswith('.svg')])} svg; skipped: {', '.join(k[8:] for k in skipped) or 'none'}")
    return 0 if ok else 2


if __name__ == "__main__":
    sys.exit(main())
