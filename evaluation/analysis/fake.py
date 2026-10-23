"""Fake-data generator: schema-correct ``corpus/manifest.jsonl``, ``results/{runs,truth,adjudication,truth_excluded,
latency}.jsonl``, ``eval.toml`` and ``cache_coherence.json`` with synthetic numbers, to exercise the whole pipeline
before real results exist.

    python -m analysis.fake --out /tmp/fake            # writes /tmp/fake/{corpus,results,eval.toml,...}
    python -m analysis --results /tmp/fake/results --truth /tmp/fake/results/truth.jsonl --out /tmp/fake/export \\
        --cache-coherence /tmp/fake/cache_coherence.json

The numbers are synthetic and meaningless. Field names and value domains follow petrivet-eval/src/record.rs,
truth.rs, adjudicate.rs and instance.rs (hashes: sha256 stands in for blake3, same 64 hex characters).
``inject_disagreements`` flips that many definite pv-dispatch answers (to test the DISAGREE path).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import random
from pathlib import Path

CLASSES = ["general", "circuit", "sm", "mg", "efc", "ac"]
CAP_MS = 60000.0
SOLVERS_B = ["pv-dispatch", "pv-general", "pv-poly", "lola", "tina"]
BOOL_Q = ["Q1", "Q3", "Q4"]


def h(*parts) -> str:
    return hashlib.sha256("\0".join(map(str, parts)).encode()).hexdigest()


class Gen:
    def __init__(self, seed: int, inject: int):
        self.rng = random.Random(seed)
        self.inject = inject
        self.manifest: list[dict] = []
        self.truth: list[dict] = []
        self.runs: list[dict] = []
        self.adj: list[dict] = []
        self.excluded: list[dict] = []
        self.truth_map: dict[tuple, object] = {}

    # --- instances ---
    def add_instance(self, source: str, name: str, cls: str, **extra) -> dict:
        iid = h("inst", source, name)
        m = {"instance_id": iid, "path": f"{source}/{name}.pnml", "source": source, "detected_class_petrivet": cls,
             "places": self.rng.randint(2, 12), "transitions": self.rng.randint(2, 20), "arcs": self.rng.randint(4, 40),
             "tokens_total": self.rng.randint(1, 6), "max_weight": 1, "targets": [], "known_answers": []}
        m.update(extra)
        self.manifest.append(m)
        return m

    def add_truth(self, m: dict, query: str, value, source: str, tid: str | None = None) -> None:
        self.truth.append({"instance_id": m["instance_id"], "query": query, "target_id": tid, "value": value, "source": source,
                           "corpus_source": m["source"], "tools": ["lola", "tina"] if source == "lola+tina" else []})
        self.truth_map[(m["instance_id"], query, tid or "")] = value

    # --- run records ---
    def record(self, campaign, m_or_id, query, tid, solver, rep, verdict, wall, **kw) -> dict:
        iid = m_or_id if isinstance(m_or_id, str) else m_or_id["instance_id"]
        r = {"run_id": h(campaign, iid, query, tid or "", solver, rep), "campaign": campaign, "instance_id": iid, "query": query,
             "target_id": tid, "solver_id": solver, "rep": rep, "verdict": verdict, "raw_hash": None, "stderr_hash": None,
             "wall_ms": wall, "cpu_user_ms": wall * 0.9, "cpu_sys_ms": wall * 0.05, "peak_rss_kib": kw.pop("rss", 20000 + self.rng.randint(0, 5000)),
             "parse_ms": None, "analyze_ms": None, "timeout_ms": None, "limit_ms": int(CAP_MS if campaign != "A" else 30000),
             "mem_limit_kib": 8 * 1024 * 1024, "exit_code": 0, "signal": None, "states": None, "edges": None, "convert_ms": None,
             "env_id": "fake-env", "verdict_differs": False, "pinned_core": None, "mem_limiter": "none", "note": None,
             "detail": None, "cmd": f"{solver} fake", "started_unix_ms": 1_700_000_000_000 + len(self.runs)}
        if verdict == "TIMEOUT":
            r["exit_code"], r["signal"] = None, 9
        r.update(kw)
        self.runs.append(r)
        return r

    def answer(self, truth_value, solver: str, p_unknown: float = 0.0, flip: bool = False) -> tuple[str, dict]:
        if self.rng.random() < p_unknown:
            return "UNKNOWN", {}
        if isinstance(truth_value, dict):
            return "COUNT", {"states": truth_value["STATES"], "edges": truth_value["TRANSITIONS"]}
        return ("TRUE" if (truth_value != flip) else "FALSE"), {}

    def adjudicate_line(self, r: dict, m: dict, tv, tsrc: str | None) -> None:
        v = r["verdict"]
        oos = r["query"] in BOOL_Q and self.truth_map.get((r["instance_id"], "Q2", "")) is False
        if v == "CRASH":
            outcome = "CRASH"
        elif oos:
            outcome = "OUT_OF_SCOPE"
        elif v in ("TRUE", "FALSE", "COUNT"):
            if tv is None:
                outcome = "NO_TRUTH"
            elif v == "COUNT":
                outcome = "AGREE" if (r["states"], r["edges"]) == (tv["STATES"], tv["TRANSITIONS"]) else "DISAGREE"
            else:
                outcome = "AGREE" if (v == "TRUE") == tv else "DISAGREE"
        else:
            outcome = v
        self.adj.append({"run_id": r["run_id"], "campaign": r["campaign"], "instance_id": r["instance_id"], "query": r["query"],
                         "target_id": r["target_id"], "solver_id": r["solver_id"], "outcome": outcome, "truth_value": tv,
                         "truth_source": tsrc, "detail": None, "answer": None if v not in ("TRUE", "FALSE", "COUNT") else v,
                         "source": m["source"], "pattern": "petrivet-alone" if outcome == "DISAGREE" else None, "verdict_differs": False})

    def emit_cell(self, campaign: str, m: dict, query: str, tid: str | None, solver: str, wall: float, tv, tsrc, *, p_unknown=0.0,
                  reps=1, **kw) -> None:
        flip = False
        if self.inject > 0 and solver == "pv-dispatch" and tv is not None and not isinstance(tv, dict) and campaign == "A":
            flip, self.inject = True, self.inject - 1
        verdict, extra = ("TIMEOUT", {}) if wall >= CAP_MS else self.answer(tv, solver, p_unknown, flip) if tv is not None else self.answer(self.rng.random() < 0.5, solver, p_unknown)
        if verdict == "TIMEOUT":
            wall = CAP_MS
        first = None
        n = reps if verdict in ("TRUE", "FALSE", "COUNT", "UNKNOWN") else 1
        for rep in range(n):
            w = wall * (1 + self.rng.uniform(-0.03, 0.03))
            rec = self.record(campaign, m, query, tid, solver, rep, verdict, w, analyze_ms=w * 0.8, parse_ms=w * 0.1 if solver.startswith("pv") else None, **extra, **kw)
            if rep == 0:
                first = rec
        self.adjudicate_line(first, m, tv, tsrc)

    # --- sources ---
    def build(self) -> None:
        rng = self.rng
        # gen: campaign A
        for cls in CLASSES:
            for i in range(20):
                m = self.add_instance("gen", f"{cls}-{i}", cls, generator_class=cls, size_bucket=rng.choice(["2-5", "6-8", "9-12"]), marking_regime="sparse")
                bounded = rng.random() > 0.1
                self.add_truth(m, "Q2", bounded, "lola+tina")
                vals = {q: rng.random() < 0.55 for q in BOOL_Q}
                for q in BOOL_Q:
                    self.add_truth(m, q, vals[q], "lola+tina")
                m["targets"] = [{"target_id": h("t", m["instance_id"], k)[:16], "kind": "QR" if k < 2 else "QC", "marking": [0, 1]} for k in range(4)]
                for t in m["targets"]:
                    if rng.random() < 0.05:
                        self.excluded.append({"instance_id": m["instance_id"], "path": m["path"], "corpus_source": "gen", "class": cls, "query": t["kind"],
                                              "target_id": t["target_id"], "target": "[0,1]", "reason": "lola and tina disagree", "results": []})
                        continue
                    self.add_truth(m, t["kind"], rng.random() < 0.5, "lola+tina", t["target_id"])
                for solver in ["pv-dispatch", "pv-general", "pv-poly"]:
                    pu = 0.0 if solver != "pv-poly" else (0.2 if cls in ("circuit", "sm", "mg") else 0.7)
                    for q in ["Q1", "Q2", "Q3", "Q4"]:
                        self.emit_cell("A", m, q, None, solver, rng.uniform(1, 30), self.truth_map[(m["instance_id"], q, "")], "lola+tina", p_unknown=pu)
                    for t in m["targets"]:
                        tv = self.truth_map.get((m["instance_id"], t["kind"], t["target_id"]))
                        self.emit_cell("A", m, t["kind"], t["target_id"], solver, rng.uniform(1, 40), tv, "lola+tina" if tv is not None else None, p_unknown=pu / 2)
        # fam: campaign B
        for cls in CLASSES:
            fam = f"fam-{cls}"
            for k in range(1, 9):
                size = 2 ** k
                m = self.add_instance("fam", f"{fam}-{size}", cls, family=fam, size_param=size)
                tv = {q: (rng.random() < 0.5) for q in BOOL_Q}
                for q in ["Q2"] + BOOL_Q:
                    self.add_truth(m, q, True if q == "Q2" else tv[q], "construction")
                for q in BOOL_Q:
                    t_true = self.truth_map[(m["instance_id"], q, "")]
                    base = 0.5 * size ** (0.4 if cls != "general" else 0.9)
                    t_gen = 5.4 * 3.2 ** k * (1.0 if cls != "general" else 1.5) * rng.uniform(0.8, 1.2)
                    t_disp = base * (1.0 if cls == "general" else 0.15) * rng.uniform(0.8, 1.2) + (1.0 if cls == "general" else 0)
                    t_disp = t_gen * rng.uniform(0.9, 1.1) if cls == "general" else t_disp
                    for solver, w, pu in (("pv-dispatch", t_disp, 0), ("pv-general", t_gen, 0), ("pv-poly", t_disp * 0.5, 0 if cls != "general" else 1.0),
                                          ("lola", t_gen * 0.3, 0), ("tina", t_gen * 0.4, 0)):
                        reps = 5 if w < 1000 else 3 if w < 10000 else 1
                        self.emit_cell("B", m, q, None, solver, w, t_true, "construction", p_unknown=pu, reps=reps,
                                       peak_rss_kib=int(2e4 + w * (3 if solver == "pv-general" else 1.5)))
        # mcc-small: B
        for i in range(30):
            cls = "general" if i % 4 else "sm"
            m = self.add_instance("mcc-small", f"M{i}", cls, mcc_model_id=f"M{i}-PT-1", mcc_family=f"M{i}")
            states = int(10 ** rng.uniform(2, 6))
            self.add_truth(m, "Q0", {"STATES": states, "TRANSITIONS": states * 3}, "mcc-oracle")
            for q in BOOL_Q + ["Q5", "Q6"]:
                self.add_truth(m, q, rng.random() < 0.5, "mcc-oracle")
            tids = []
            if i < 10:
                for k, kind in enumerate(["QR", "QR", "QC", "QC"]):
                    tid = h("tt", i, k)[:16]
                    tids.append((tid, kind))
                    self.add_truth(m, kind, rng.random() < 0.5, "lola+tina", tid)
            for solver in SOLVERS_B:
                scale = {"pv-dispatch": 1.0, "pv-general": 1.1, "pv-poly": 0.3, "lola": 0.4, "tina": 0.5}[solver]
                for q in ["Q0"] + BOOL_Q + ["Q5", "Q6"]:
                    w = states * 0.02 * scale * rng.uniform(0.5, 2)
                    if solver == "pv-poly" and q in ("Q0", "Q6"):
                        self.emit_cell("B", m, q, None, solver, 1.0, self.truth_map[(m["instance_id"], q, "")], "mcc-oracle", p_unknown=1.0)
                        continue
                    kw = {"rss": int(3e4 + states * 0.3 * (1 if solver != "pv-general" else 1.3))}
                    self.emit_cell("B", m, q, None, solver, w, self.truth_map[(m["instance_id"], q, "")], "mcc-oracle", p_unknown=0.1 if solver == "pv-poly" else 0, reps=3 if w < 10000 else 1, peak_rss_kib=kw["rss"])
                for tid, kind in tids:
                    self.emit_cell("B", m, kind, tid, solver, rng.uniform(5, 3000) * scale, self.truth_map[(m["instance_id"], kind, tid)], "lola+tina", reps=3)
        # mcc-large: B
        for i in range(10):
            m = self.add_instance("mcc-large", f"L{i}", "general", mcc_model_id=f"L{i}-PT-9", mcc_family=f"L{i}")
            for q in ["Q1", "Q3", "Q5"]:
                self.add_truth(m, q, rng.random() < 0.5, "mcc-oracle")
                for solver in SOLVERS_B:
                    w = CAP_MS if solver in ("pv-general", "pv-dispatch") and rng.random() < 0.7 else rng.uniform(100, 20000)
                    self.emit_cell("B", m, q, None, solver, w, self.truth_map[(m["instance_id"], q, "")], "mcc-oracle", p_unknown=0.5 if solver == "pv-poly" else 0)
        # baselines (campaign B and D)
        for camp in ("B", "D"):
            for solver in SOLVERS_B:
                for rep in range(3):
                    self.record(camp, "BASELINE", "Q1", None, solver, rep, "TRUE", {"lola": 3.0, "tina": 4.0}.get(solver, 6.0) + rng.random(), rss=8000)
        # campaign C: pv-dispatch with timeout on large fam/mcc-large cells
        cands = [m for m in self.manifest if (m["source"] == "fam" and m.get("size_param", 0) >= 128) or m["source"] == "mcc-large"][:12]
        for m in cands:
            for t in (10, 100, 1000):
                over = rng.lognormvariate(1.0, 1.0)
                for rep in range(3 if t == 10 else 1):
                    self.record("C", m, "Q1", None, "pv-dispatch", rep, "UNKNOWN" if rng.random() < 0.8 else "TRUE", t + over + 8, timeout_ms=t, analyze_ms=t + over, parse_ms=2.0)
        # campaign D: subprocess latency of Q1/Q3 on small gen nets
        small = [m for m in self.manifest if m["source"] == "gen"][:20]
        for m in small:
            for q in ("Q1", "Q3"):
                for solver in ("pv-dispatch", "lola", "tina"):
                    self.record("D", m, q, None, solver, 0, "TRUE", {"pv-dispatch": 7.0, "lola": 5.0, "tina": 6.0}[solver] + rng.random())

    def latency(self) -> list[dict]:
        """Rows in the schema of petrivet-eval/src/latency.rs (LatencyRecord)."""
        out = []
        first = True
        for variant, base in (("pv-inproc-parsed", 3.0), ("pv-inproc-cached", 0.4), ("pv-inproc-parse", 40.0), ("pv-dispatch", 7000.0), ("lola", 5000.0), ("tina", 6000.0)):
            for k in range(200):
                us = base * (20 if k == 0 else 1) * self.rng.lognormvariate(0, 0.2)
                out.append({"campaign": "D", "kind": "call", "variant": variant, "query": "Q1", "instance_id": "x", "class": "sm", "places": 4, "transitions": 3,
                            "position": k, "phase": "cold" if k == 0 else "steady", "ns": int(us * 1000), "ms": us / 1000, "verdict": "TRUE",
                            "process_first": first and variant.startswith("pv-inproc") and k == 0, "aborted": False, "exit_code": None, "signal": None,
                            "detail": None, "cmd": None, "env_id": "fake-env", "started_unix_ms": 0})
                if variant.startswith("pv-inproc") and k == 0:
                    first = False
        out.append({"campaign": "D", "kind": "call", "variant": "lola", "query": "Q3", "instance_id": "y", "position": 0, "phase": "cold", "ns": 1, "ms": 0.0,
                    "verdict": "CRASH", "process_first": False, "aborted": True})
        for name in ("baseline-noop", "baseline-pv-version"):
            for k in range(200):
                out.append({"campaign": "D", "kind": "baseline", "variant": name, "query": None, "instance_id": None, "position": k, "phase": "cold" if k == 0 else "steady",
                            "ns": int((1000 + self.rng.random() * 100) * 1000), "ms": 1.0, "verdict": "TRUE", "process_first": False, "aborted": False, "env_id": "fake-env"})
        return out


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(r, sort_keys=True) + "\n" for r in rows))


def generate(out: Path, seed: int = 1, inject_disagreements: int = 0) -> Path:
    g = Gen(seed, inject_disagreements)
    g.build()
    write_jsonl(out / "corpus" / "manifest.jsonl", g.manifest)
    res = out / "results"
    write_jsonl(res / "runs.jsonl", g.runs)
    write_jsonl(res / "truth.jsonl", g.truth)
    write_jsonl(res / "adjudication.jsonl", g.adj)
    write_jsonl(res / "truth_excluded.jsonl", g.excluded)
    write_jsonl(res / "latency.jsonl", g.latency())
    (out / "eval.toml").write_text("[run]\nwall_cap_secs = 60\na_wall_cap_secs = 30\n\n[e3]\novershoot_threshold_ms = 50.0\n")
    (out / "cache_coherence.json").write_text(json.dumps({"differences": 0, "nets": 200}) + "\n")
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--inject-disagreements", type=int, default=0)
    a = ap.parse_args()
    generate(a.out, a.seed, a.inject_disagreements)
    print(f"fake data written to {a.out}")


if __name__ == "__main__":
    main()
