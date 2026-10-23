# petrivet evaluation: specification (v2)

Status: DRAFT for review. Nothing in this directory is thesis content.
Supersedes v1 (2026-10-20). Changes since v1 are listed in section 15.

Goal: one automated pipeline whose output (tables, figures, numbers) is read directly by the thesis chapter,
and which doubles as a long-term regression suite for the library.
The evaluation is broad over claims and shallow over nets, so that a full run takes hours, not days.

## 0. Terminology

- **Instance**: one Petri net with its initial marking, stored as a PNML file.
- **Query**: one question asked of an instance (section 3).
- **Solver**: anything that takes (instance, query) and returns a verdict (section 6).
- **Run**: one solver on one (instance, query) under one set of limits; one line in `runs.jsonl`.
- **Truth**: the trusted verdict for an (instance, query), with a recorded source (section 5).
- **Source**: where an instance comes from: `gen`, `fam`, `mcc-small`, `mcc-large`. Sources are never pooled in a table.

## 1. Claims, hypotheses, non-claims

Hypotheses are fixed before the first measured run. Changing one afterwards is logged as an amendment at the end of this file.

| Id | Claim (research question) | Hypothesis | Experiment |
|---|---|---|---|
| C1 | Answers are correct (RQ3) | **H1**: zero definite disagreements between `pv-dispatch` and truth, on all conclusive instances of `gen`, `mcc-small`, `mcc-large`, for queries Q0-Q6. **H1b**: same for QR and QC on `gen`. | E1, E5 |
| C2 | Dispatching to the cheapest applicable procedure pays off (RQ2) | **H2**: on instances of a restricted structural class, `pv-dispatch` is faster than `pv-general`; paired effect positive (Wilcoxon signed-rank, alpha 0.05, Holm-corrected across classes; Hodges-Lehmann estimate reported). | E2 |
| C3 | The caller can restrict runtime cost (RQ2, R3) | **H3a**: there are instances where `pv-poly` returns a definite, correct verdict within the cap and `pv-general` times out. **H3b**: the library `timeout` returns close to its deadline. The threshold for "close" is NOT fixed yet; it is set from the by-hand pilot (section 9) and frozen in `eval.toml` before E3 runs. | E3 |
| C4 | Standing relative to the state of the art, and embedding latency (RQ3, R1, R5) | **H4**: for small nets, in-process latency is lower than any subprocess-based tool. | E4, E6 |

Non-claims (stated in the chapter): petrivet does not outperform LoLA or TINA on large general nets;
no claim about unbounded nets except Q2; no performance claim for QR and QC against reference tools beyond what E5 shows.

Explicitly outside this evaluation (see section 14): R1-R8 self-rating, integration case study, agent study, API ergonomics, WASM.

## 2. Principles

1. One record shape for every run; every analysis is a query over the same table.
2. The corpus is generated once, written to files, hashed and archived. Runs read files, never generators.
3. Solvers are black boxes behind adapters.
4. Limits are enforced from outside the process wherever wall time is measured. The library's own `timeout` is a subject of study (E3), never the measurement instrument.
5. Resumable and idempotent: a stage only does missing work, keyed by content hashes.
6. Everything environment-dependent is recorded with the run.
7. Sources are never pooled: every table and test is per source.
8. Selection rules are code with a seed, committed with their output. No hand-picking of instances.

## 3. Queries

Defined over the reachability set R(N, M0). An adapter implements exactly these or reports `NOT_SUPPORTED`.

| Id | Query | Result | MCC examination (truth source on `mcc-*`) |
|---|---|---|---|
| Q0 | number of reachable markings and edges (bounded nets) | integer pair | StateSpace (STATES, TRANSITIONS) |
| Q1 | deadlock-free: every marking in R enables a transition | bool | ReachabilityDeadlock, **inverted** (oracle TRUE means a deadlock exists) |
| Q2 | bounded: R is finite | bool | none |
| Q3 | live (L4): from every marking in R every transition can be enabled | bool | Liveness |
| Q4 | quasi-live (L1): every transition is enabled somewhere in R | bool | QuasiLiveness |
| Q5 | 1-safe: no marking in R puts more than one token on a place | bool | OneSafe |
| Q6 | stable place: some place has the same token count in all of R | bool | StableMarking |
| QR | target marking T is reachable: T in R | bool | none |
| QC | target marking T is coverable: some marking in R is >= T | bool | none |

Applicability by source:

| Query | gen | fam | mcc-small | mcc-large |
|---|---|---|---|---|
| Q0 | no | no | yes | no |
| Q1, Q3, Q4 | yes | yes | yes | yes (where the oracle is definite) |
| Q2 | yes | yes | no | no |
| Q5 | no | no | yes | yes (where definite) |
| Q6 | no | no | yes | no |
| QR, QC | yes | no | yes (section 5.5) | no |

Q0 and Q6 have only the general procedure in petrivet; they enter E1 and E4 but not the dispatch ablation.
Out of scope: CTL/LTL, per-place bounds (UpperBounds), per-transition liveness levels, unbounded nets for Q1, Q3, Q4 (truth is inconclusive there and such instances are counted and excluded, not guessed).

## 4. Components to build

Two repositories. The library repository (`github.com/MichaelOwenDyer/petrivet`) holds the library, the CLI and the library's own flake. The thesis repository holds everything else, under `evaluation/`, which is its own cargo workspace and its own flake. The thesis workspace depends on petrivet through its GitHub repository (a git dependency in cargo, a `github:` input in the flake), not through paths on one machine; a git-ignored `evaluation/.cargo/config.toml` (template: `config.toml.example`) can patch the dependency to a local checkout while library changes are not pushed yet.

```
petrivet repository
  petrivet/                library (frozen during the evaluation run, after the deadline fixes of section 16 step 2)
  petrivet-cli/            --timeout-ms, properties reach/cover with --target, no method output
  flake.nix, nix/package.nix   petrivet's own flake, independent of the evaluation

thesis repository, evaluation/
  Cargo.toml               workspace: petrivet-eval, petrivet-testkit
  petrivet-testkit/        lib + `gen` binary: generators (with per-class construction assertions), target proposer, manifest writer. No authority over truth (section 7)
  petrivet-eval/           bin: stages select / fetch / histogram (done), truth / run / adjudicate / analyze / export (to do)
  flake.nix, nix/          the evaluation environment: LoLA (nix/lola.nix), TINA (nix/tina.nix, unfree, only that package), petrivet from the GitHub flake input, devShell with all three
  eval.toml                every parameter (seeds, caps, sizes, reps, thresholds, solver commands, paths)
  corpus/  results/  export/   generated; layout in sections 5 and 8
  adapters                 one small module per reference tool (command template, formula mapping, output parser), inside petrivet-eval
mcc-tests (petrivet-mcc)   not a dependency: the oracle format is parsed by petrivet-eval; setup.sh was the model for `fetch`. NOT reused: runner.rs run_analysis (see below), the orchestrator, the BenchKit binary.
```

Why `run_analysis` is not reused: it calls the explicit-state procedure directly for Q0, Q4 and Q6 and `deadlocks()` for Q1, so
those examinations never exercise the dispatcher; it has no timeout; and the existing test harness counts "not computable" as a failure.
The comparison logic (`compare_verdicts`) and the oracle parser are reused.

CLI contract (to implement):

```
petrivet analyze FILE -p df|bounded|live|quasi-live|statespace|onesafe|stable|reach|cover
          [-m dispatch|poly|general] [--timeout-ms N] [--target FILE.json] --json
```

- Modes: `dispatch` = library default effort; `poly` = efficient effort only (may answer `unknown`); `general` = explicit-state procedure only.
  For `reach` and `cover`: `dispatch` = the library's CEGAR pipeline, `general` = explicit-state search; `poly` is not applicable.
- `--target`: JSON map from place id to token count.
- JSON fields: tool, version, file, property, mode, verdict, class, places, transitions, arcs, parse_ms, analyze_ms, timeout_ms, optional states, edges.
- Verdict `unknown` means the library gave up (deadline or no applicable procedure). A kill by the runner is recorded by the runner as `TIMEOUT`, never by the CLI.
- `onesafe` is `is_bounded().k(1)`; `stable` and `statespace` are general-only.

Implemented commands of `petrivet-eval` (workspace `evaluation/`, parameters in `evaluation/eval.toml`; build with the Z3 library path from `evaluation/.cargo/config.toml`; run from `evaluation/`):

- `petrivet-eval oracle`: downloads the MCC verdict archive (`[fetch] oracle_url`, cached as `corpus/mcc/oracle.tar.gz`) and lays the `.out` files out as `corpus/mcc/oracle/<ModelId>/<ModelId>-<EXAM>.out`. Idempotent. Needed once on a fresh clone.
- `petrivet-eval doctor`: checks config, oracle model count, `select --check`, sha256 of every selected model, and reports which of `lola`, `tina`, `ndrio`, `selt`, `petrivet` are on PATH (missing tools are not fatal; data check failures exit non-zero).
- `petrivet-eval select [--check]`: spec 5.2 from the 2026 oracle verdict files; writes `corpus/mcc/selection.json` (parameters, pool counts, per family all small candidates ordered nearest-to-median first, ranked large candidate lists per examination). Deterministic; `--check` writes nothing and exits non-zero if the fresh selection differs from the file on disk.
- `petrivet-eval fetch [--accept-changes]`: draws the models by walking the candidate lists and downloading fresh from the 2026 archive (`curl` and `tar`), resumable. A candidate counts only if petrivet parses it (connected, ordinary), it is not colored, and (large) its PNML is within the size cap; every rejection is recorded with its reason in `selected.json`. mcc-small uses two passes (target 90, at most 2 per family, rule in `eval.toml`); mcc-large takes 5 per examination, a family at most once. Each file is compared with the 2025 archive when `oracle_2025_zip` is configured (otherwise the recorded value is kept). After fetching, every sha256 is compared with the existing `selected.json`; a mismatch or a no longer drawn model is reported and fails the command unless `--accept-changes` is given. Writes `corpus/mcc/selected.json`.
- `petrivet-eval histogram`: parses every selected model with petrivet and writes `corpus/mcc/histogram.json` (class, size, weight and initial-marking statistics per stratum, parse failures).

## 5. Corpus

### 5.1 Sources

| Source | Content | Size | Purpose | Truth |
|---|---|---|---|---|
| `gen` | random small nets per structural class | about 1800 | correctness, E1 and E5 | LoLA and TINA agree (section 7) |
| `fam` | parametric scaling families, sizes growing geometrically | 6 classes x about 8 sizes | ablation and scaling, E2, E3a | by construction |
| `mcc-small` | MCC 2026 non-colored models, state space known and at most 1e6 | about 90 | known-answer correctness, comparison, E1, E4 | MCC oracle |
| `mcc-large` | MCC models with a definite verdict but state space unknown or above 1e6 | about 20 | the only place cheap procedures can show benefit against a known verdict, E3a (exploratory) | MCC oracle |

Facts about the MCC 2026 oracle (counted from the downloaded verdict files, 2026-10-22): 1953 models, 1681 non-colored.
The `mcc-small` pool (known state space at most 1e6) has 199 models from 92 families, with verdicts
Liveness 136 false / 60 true, QuasiLiveness 155 true / 43 false, ReachabilityDeadlock 95 true / 104 false, OneSafe 121 true / 78 false. The mix is not one-sided.
The `mcc-large` pool (state space unknown or above 1e6) has 1482 models, of which more than 1300 carry a definite verdict for each of Liveness, QuasiLiveness, ReachabilityDeadlock and OneSafe.
Only the verdict files of the 2026 oracle are downloaded for selection (about 50 MB). Model PNML files are downloaded fresh from the 2026 archive for the selected models only (about 110), not unpacked from the 2025 archive,
and each is compared with the 2025 copy where one exists, to report how many models changed between years.

### 5.2 Selection of the MCC subsets (code in `petrivet-eval select`, seeded, output committed)

- Pool: model id without `-COL-`, no `iscolored` marker, at least one definite verdict (not `?`) among SS, RD, QL, SM, L, OS.
- `mcc-small`: pool with definite STATES at most 1e6. Per family, take the model at the median of the family's state counts (deterministic; avoids both trivial and extreme models). One per family.
- `mcc-large`: pool with STATES `?`/absent or above 1e6 and a definite verdict for at least one of L, RD, QL, OS; PNML file at most 5 MB. Per family at most one; stratified so that each of the four examinations has about five models; seeded.
- No filtering on whether petrivet, LoLA or TINA can parse the model. Parse failures are counted (A10 coverage), not hidden.
- Weighted arcs occur in MCC models (15 of the 92 selected small models, 1 of 20 large; generated and family nets use weight 1 only). **petrivet's PNML importer currently ignores `<inscription>` values**, so a weighted net is read as an ordinary net without any error, and every verdict on it concerns a different net. Until the importer rejects weighted arcs, these models must be excluded from E1, E2 and E4 (counted as coverage, "unsupported: weighted arcs") and must never be counted as petrivet disagreements. Also stated as a threat.
- petrivet admits only connected nets: 10 of 92 small and 1 of 20 large selected models fail to import for that reason (counted as coverage, not hidden).
- Expectation to verify first: the subset is mostly class `General`. The class histogram of the subset is the first output of `select`, before any run.

### 5.3 Generators (`gen`)

Classes: general, circuit, state machine (SM), marked graph (MG), extended free choice (EFC), asymmetric choice (AC),
built by construction following the generator specification the author supplied (verbatim in `petrivet-testkit/GENERATORS.md`).
Decisions the supplied specification left open:

1. **Initial marking**: regime in {sparse, medium, dense}, at most 2 tokens per place, at most 6 tokens in total. Regime recorded per instance.
2. **Connectivity**: petrivet admits only connected nets. Disconnected candidates are rejected and resampled (no repair). The rejection rate per class is reported.
3. **No source or sink transitions** in `gen`: every transition has at least one input and one output place (repair preserving the class).
4. **Intended versus detected class**: each generator builds its class by construction and asserts the defining property of the class in a few lines (for example, every transition of a state machine has exactly one input and one output place). The intended class is stored and petrivet's detected class is compared with it. For a non-general intended class a mismatch is investigated by hand (a generator bug or a petrivet bug), which also tests petrivet's class detection. A general net that happens to belong to a smaller class is not a mismatch.
5. **Reporting hierarchy**: circuit is in SM and MG; SM and MG in free choice; free choice in EFC; EFC in AC; AC in general. Report by the most specific detected class.
6. **Density and arity**: arc probability for general nets; maximum preset and postset size for EFC and AC.
7. **Verdict balance**: after generation report the share of live, deadlock-free and bounded instances per class (from truth). Tune the generator parameters (regime, density) by hand, once, until each verdict is at least 20 percent of conclusive instances per class and query, where structurally possible. Where it is not (MG and SM), say so. There is no automatic re-weighting loop.
8. **Weights**: ordinary arcs only.

Size buckets: |P| in 2-5, 6-8, 9-12; |T| between |P|/2 and 2|P|. About 100 nets per (class, bucket) cell, 6 x 3 x 100.
Seed per net: `hash(master_seed, class, bucket, index)`, so any net regenerates alone and adding nets does not change existing ones.
The corpus is written to disk and hashed; the seed is provenance, not the means of reproduction.

### 5.4 Families (`fam`)

One family per class where a scalable construction exists (circuit ring, SM cycle with branching, live and dark marked graphs, free-choice and asymmetric-choice patterns from `benches/analysis.rs`).
Each family has a size parameter n in a geometric series (for example 2^k). The family generator also states the verdict of each query as a function of n, which is the truth.
Size range is fixed after the by-hand pilot so that the smallest size is trivial and the largest times out for `pv-general`. Known issue: the asymmetric-choice bench rung hangs; the external cap handles it and it is reported as a timeout, not worked around.

### 5.5 Target markings for the reachability and coverability queries

Two queries need a target marking as input: "is this marking reachable?" (QR) and "can this marking be covered, that is, is some reachable marking at least as large in every place?" (QC).
The testkit only proposes candidate targets. The right answer for each candidate comes from outside the testkit (section 7), so a testkit bug cannot make petrivet look correct.

**Proposing candidates** (same procedure for generated nets and for the MCC models used here):
- Reachable candidates: the marking at the end of a random firing sequence of random length. The firing sequence is stored. Replaying it proves that the marking is reachable, costs a ten-line firing function and needs no tool; it is kept as a bonus check, not as the source of truth.
- QC candidates are derived from reachable markings by removing a few tokens (coverable) or by adding tokens to a place beyond what any reachable marking was seen to have (probably uncoverable).
- Further candidates are markings drawn at random from a bounded box around the initial marking (at most two tokens per place more than in the initial marking). Most are unreachable and some are reachable by chance; the label decides.
- Per net: up to four candidates per query kind, balanced between expected-yes and expected-no.

**Labelling** is by agreement of LoLA and TINA (section 7). A candidate on which they disagree, or on which one is inconclusive within the labelling limit, is dropped and counted. The drop decision is made before any petrivet answer is seen.

**Where candidates come from**: all `gen` nets, and up to 30 `mcc-small` models. LoLA and TINA later run on the same targets as competitors (E5b); the labelling runs and the timed runs are separate.

**Stratification.** Targets are reported as yes (reachable, coverable) versus no (unreachable, uncoverable). A finer split by how much refinement petrivet needs would be informative, but the library's observer reports one event per added lemma, not per round, and enabling it changes what the solver computes. So that split is not available today. If the library later exposes a refinement counter it is added; otherwise the chapter reports the yes/no split only and says that hard unreachable targets are not singled out.

**Limitations, stated next to every affected table**:
- Labels on the MCC models depend on two tools agreeing within a generous limit, so only targets that the tools can decide are included. This is biased towards easier cases.
- There is no certificate for unreachable targets on realistic nets, so their correctness evidence is the agreement of two third-party tools, not a proof.

### 5.6 Instance record, `corpus/manifest.jsonl`

| Field | Notes |
|---|---|
| instance_id | blake3 of the canonical PNML bytes |
| path | relative path to the PNML |
| source | gen, fam, mcc-small, mcc-large |
| generator_class / detected_class_testkit / detected_class_petrivet | gen only for the first |
| places, transitions, arcs, tokens_total, max_weight | |
| marking_regime, size_bucket, seed, gen_version | gen only |
| family, size_param | fam only |
| mcc_model_id, mcc_family | mcc only |
| targets | list of {target_id, kind QR/QC, marking, firing_sequence (when the candidate came from a random walk)}; the label lives in `truth.jsonl` |

`corpus/CORPUS.sha256` hashes every file and the manifest; `verify` fails the pipeline if anything changed.

## 6. Solvers

| solver_id | What | Role |
|---|---|---|
| `pv-dispatch` | petrivet CLI, `-m dispatch` | subject |
| `pv-general` | petrivet CLI, `-m general` | ablation baseline |
| `pv-poly` | petrivet CLI, `-m poly` (efficient effort) | cost-control claim |
| `pv-inproc` | the library called from the evaluation process | correctness (A) and latency (D) only, never wall-time comparisons |
| `lola` | LoLA, version pinned | reference, truth for `gen`, comparison |
| `tina` | TINA toolbox, version pinned | reference, truth for `gen`, comparison |

Adapters: each reference tool has a module mapping each query to a command or formula (following the MCC definitions of the examinations),
a conversion step if the tool does not read PNML (cached by instance hash, outside timed sections, conversion failures counted), and an output parser.
Conversion path for LoLA: via TINA's `ndrio` (to verify on the first net; an unverified `mcc` command was mentioned and is not relied upon).
LoLA 2.0 has no whole-net bounded, live or quasi-live query: the adapter combines per-place or per-transition formulas (see `adapters-notes.md`) and reads the verdict from LoLA's JSON output, because its exit code is 0 for both answers. Query support per tool is whatever CALIBRATE (below) establishes; the expected table is LoLA and TINA: Q0-Q6, QR, QC; to be confirmed.
Packaging: no nixpkgs package exists for these tools under the obvious names (`nix search` on 2026-10-22), so each is packaged as a small derivation. Cap: half a day per tool. If a tool is dropped, the chapter says so.

### CALIBRATE

Purpose: make sure an adapter says what we think it says (wrong formula, flipped polarity, different definition of "live"), so that a disagreement later is a finding and not an adapter bug.
- About 30 tiny nets with answers derived by hand for every applicable query (existing test nets and textbook examples). Every solver, including `pv-*`, runs them.
- A solver that disagrees with the hand answer is fixed. If its definition genuinely differs, the query is `NOT_SUPPORTED` for that solver and the chapter says why.
- Second, free check at scale: LoLA and TINA against the MCC oracle on `mcc-small` for Q1, Q3, Q4, Q5 (and Q0, Q6 where supported). Agreement rates are reported. A systematic mismatch is an adapter fault until shown otherwise.
- The evaluation does not proceed to RUN until calibration passes.

## 7. Truth

Principle: the testkit never decides a verdict. Truth comes from the MCC oracle or from two mature third-party tools agreeing.

| Source | Rule |
|---|---|
| `mcc-*` | MCC oracle verdict, `?` skipped, never guessed |
| `fam` | by construction |
| `gen`, and QR/QC targets on `gen` and `mcc-small` | LoLA and TINA agree, and neither is inconclusive within the labelling limit (generous and untimed, for example 10 minutes) |

If LoLA and TINA disagree, or one is inconclusive, the instance (or target) is excluded and counted. The exclusion is decided in the TRUTH stage, before any petrivet answer is looked at, so petrivet's results cannot bias it.
Exclusion counts per query are reported; large counts for some class are themselves a finding about the tools or the adapters.
When a petrivet answer conflicts with a truth that two tools agree on, the instance is investigated by hand with a throwaway script (a small explicit-state search on that one net). That script is a debugging aid, not part of the pipeline.
If packaging of LoLA or TINA fails the day-1 gate, a small explicit-state evaluator is written as the replacement truth source for `gen`. That is the only case in which the testkit would gain authority, and the chapter would then say so.
Truth file: `results/truth.jsonl`, one line per (instance, query, target?): value, source, tools consulted.

## 8. Run record and measurement protocol

### 8.1 Run record (`results/runs.jsonl`)

| Field | Type | Notes |
|---|---|---|
| run_id | string | blake3(campaign, instance_id, query, target_id, solver_id, rep) |
| campaign | enum | A, B, C, D |
| instance_id, query, target_id?, solver_id, rep | | |
| verdict | enum | TRUE, FALSE, COUNT(states,edges), UNKNOWN (gave up), TIMEOUT (killed), MEMOUT, NOT_SUPPORTED, ERROR, PANIC, SHARD_FAILED |
| raw_hash, stderr_hash | string | raw output stored by hash under `results/raw/` |
| wall_ms | float | spawn to exit (subprocess runs) |
| cpu_user_ms, cpu_sys_ms, peak_rss_kib | | from wait4 rusage of the child |
| parse_ms, analyze_ms | float? | as reported by petrivet (in-process timers) |
| timeout_ms | int? | internal `--timeout-ms` given to petrivet, campaign C |
| limit_ms, mem_limit_kib | int | external limits in force |
| exit_code, signal | | |
| states, edges | int? | Q0 |
| convert_ms | float? | outside timing |
| env_id | string | key into `env.json` |

`env.json`: CPU model, pinned cores, governor, turbo state, kernel, RAM, versions and hashes of every solver, rustc/cargo versions, git sha of the workspace, hash of `eval.toml`, hash of the corpus, timestamp.

### 8.2 Machine and limits

- Linux box only (i7-4790K, 32 GB, NixOS). Never measure on the Mac.
- Governor `performance`, turbo off during runs, jobs pinned with `taskset` to separate physical cores, at most 3 jobs in parallel, hyper-thread siblings idle, 8 GiB memory limit per job.
- Build: `cargo build --release` from the flake; the exact binary hash is in `env.json`.
- Wall cap for campaign B: 60 s per run (all sources). Timeouts count as 2 x cap for PAR-2. Campaign A has no wall verdicts; shards have a safety cap.
- Repetitions (campaign B only): 5 if the first run takes under 1 s, 3 if under 10 s, otherwise 1. Timeouts are not repeated. Repetitions double as a determinism check: a verdict that differs between repetitions is flagged as a finding.
- Order: runs are shuffled with a seed and solvers are interleaved per instance, so thermal and cache drift does not favour a solver. A warm-up pass reads every instance once before measuring (cold-cache effects are a legitimate condition but not what is studied).
- Startup baseline: every solver gets a no-op run on a trivial net, once per campaign.

## 9. Campaigns

### A: correctness (in-process, `gen`)
- For each net, query, and mode in {dispatch, general, effort-limited}: parse the PNML, build a fresh `PetriNet`, call the library inside `catch_unwind`, append a run (`pv-inproc`). A panic is recorded as `PANIC` with its message (a finding, never discarded).
- Fresh object per call: the library caches boundedness, liveness and deadlock results per `PetriNet`, so reuse would make results order-dependent.
- Cache-coherence pass: one `PetriNet` per net, all queries in sequence and in a shuffled order; verdicts must equal the fresh results. Any difference is a finding.
- Sharding: nets are processed in shards of about 100 by child processes with a safety timeout. A dead shard is `SHARD_FAILED` and is bisected on resume. This protects against abort, stack overflow, and hangs, which in-process execution would otherwise turn into a lost run.
- CLI smoke: 5 percent of the nets (seeded) also run through the CLI; verdicts must equal the in-process ones. This covers the CLI's flag and verdict handling, which in-process does not.
- Not measured here: time and memory.

### B: performance and comparison (subprocess, `fam`, `mcc-small`, `mcc-large`)
- Solvers: `pv-dispatch`, `pv-general`, `pv-poly`, `lola`, `tina`. Queries per section 3.
- Expected run count: about 600 (instance, query) pairs x 5 solvers x about 3 repetitions on average; upper bound of runtime if 25 percent of solver-runs hit the cap is about 4 hours with 3 parallel jobs. Recompute after the pilot.

### C: timeout behaviour (E3b)
- Instances: from B, those where the unconstrained `pv-dispatch` run took more than 2 x the largest t, so the deadline actually triggers.
- `pv-dispatch` with `--timeout-ms t`, t in {10, 100, 1000}, one run per cell (3 repetitions where under 1 s).
- Measures: `overshoot = analyze_ms - t` (absolute, parse excluded and stated), verdict (UNKNOWN vs definite), external wall as a sanity check.
- The by-hand pilot comes first (below), then the threshold is frozen. C itself is then part of the pipeline.

### D: latency (E6)
- Instances: 20 small `gen` nets per class (about 120), queries Q1 and Q3.
- Variants: `pv-inproc` with the net already parsed (cold first call and steady state reported separately), `pv-inproc` including PNML parse, `pv-dispatch` CLI subprocess, `lola`, `tina`; each query repeated in a sequence of 200; startup baseline reported.
- Expectation: microseconds in-process, milliseconds for subprocess tools. The result mostly reflects the integration mode (process start, Z3 load), not the algorithm. The chapter says exactly that.

### The pilot (manual, one-off, not a pipeline stage)
Before freezing `eval.toml`: run C on about 10 hand-picked instances, look at the overshoot, and if there are large outliers run `perf` on them once to see which phase holds the thread (LP solve, siphon enumeration, exploration between checks, parse).
The pilot also fixes the family size ranges, the cap and repetition counts. Its data is stored in `results/pilot/`, labelled exploratory, and not used in any reported test.
Output of the pilot: the H3b threshold (or a decision to report only the distribution), written into `eval.toml` and committed before any campaign runs.

## 10. Adjudication (`results/adjudication.jsonl`)

One line per run joined with truth. Outcomes: AGREE, DISAGREE, UNKNOWN, TIMEOUT, MEMOUT, ERROR, PANIC, NOT_SUPPORTED, INCONCLUSIVE_TRUTH.
Only two opposite definite verdicts can disagree. UNKNOWN, TIMEOUT and the rest never count as disagreement and are reported as coverage.

The pipeline does not determine the cause of a disagreement. It classifies the pattern automatically:

| Pattern | Candidate cause |
|---|---|
| petrivet alone differs from truth that LoLA and TINA both give | petrivet bug |
| LoLA and TINA differ from each other | semantics or conversion difference |
| petrivet agrees with one tool, the other differs | investigate case by case |
| a tool differs from the MCC oracle | adapter or conversion fault, until shown otherwise |

For each DISAGREE the pipeline writes a reproducer (instance file, exact commands, raw outputs). A human then shrinks the net, checks it with the throwaway explicit-state script, and sets the triage tag:
petrivet-bug, reference-bug, semantics-difference, generator-bug, conversion-bug. All triaged disagreements are listed in an appendix, including those that turn out not to be petrivet's fault.

## 11. Analyses (generated, no manual numbers)

| Id | Question | Method |
|---|---|---|
| E1 | Agreement with truth | per source x class x query: n conclusive, n agree, n disagree, inconclusive rate; for zero disagreements the one-sided 95 percent Clopper-Pearson upper bound on the error rate; separate row per source |
| E1b | Cache coherence and CLI smoke | counts of differences (expected zero) |
| E2 | Dispatch ablation | `fam` only (`gen` nets are too small for timing): per instance median time of `pv-dispatch` vs `pv-general`, paired; Wilcoxon signed-rank per class, Holm-corrected; Hodges-Lehmann estimate and median ratio with IQR; instances where either times out are censored at the cap, with the count reported and a sensitivity run excluding them; frontier = largest size solved within the cap; peak RSS compared likewise. On `mcc-small`: only the share of instances where dispatch differs from general (class detection and `pv-poly` definite) |
| E3a | Cost control by effort | share of `pv-poly` runs that are definite and correct, UNKNOWN share, instances where `pv-poly` answers and `pv-general` times out (`fam`, `mcc-large`; `mcc-large` flagged exploratory) |
| E3b | Cost control by timeout | distribution of overshoot per instance family and procedure; share within the (frozen) threshold; the instances and phases responsible for the largest overshoots; share UNKNOWN vs definite per t |
| E4 | Comparison to reference tools | per query and source: solved count, PAR-2, cactus plot; coverage (supported, definite, unknown, timeout, error) |
| E5a | QR/QC correctness | `gen`: agreement with truth, split yes / no (section 5.5) |
| E5b | QR/QC comparison | `mcc-small` targets: solved count and time of `pv-dispatch` vs `pv-general` vs `lola` vs `tina`; with the limitation of section 5.5 stated beside every table |
| E6 | Latency | p50, p95, p99 per variant, startup baseline separate |
| E7 | Memory | peak RSS distribution; bytes per state = (peak RSS minus no-op baseline) / states on Q0 runs |

Statistics rules: medians and IQR for timings; no parametric tests on timings; paired nonparametric tests for head-to-head comparisons; PAR-2 with the same cap for every tool;
all tests listed with their correction method. If a hypothesis is not supported, that is reported as such.

## 12. Data flow into the thesis

```
eval.toml --+
            v
 select -> corpus/manifest.jsonl + corpus/*.pnml + CORPUS.sha256
 truth  -> results/truth.jsonl
 run    -> results/runs.jsonl, results/raw/, results/env.json
 adjudicate -> results/adjudication.jsonl, results/reproducers/
 analyze -> export/numbers.json, export/*.csv, export/figures/*.svg
```

- `export/` is committed into the thesis repository (`MastersThesis/evaluation/export/`). `runs.jsonl` and `raw/` are archived (hash recorded in `numbers.json`), not necessarily committed.
- `numbers.json` holds every number the prose cites, under stable keys (for example `e1.mcc_small.n_conclusive`). Prose refers to them; no number is copied by hand.
- Tables: `export/*.csv`, rendered in Typst by `csv()` plus a table template, so the chapter and the data cannot drift apart.
- Figures: SVG produced by the analysis scripts (cactus plots, paired scatter, scaling lines, overshoot histograms), included with `image()`.
- Analysis scripts are Python (polars, scipy), pinned in the flake. One command regenerates all of `export/` from `runs.jsonl` and `truth.jsonl`.
- Typst is told the generation time and `eval.toml` hash through `numbers.json`, and the chapter prints both in a footnote, so a reader can tell which run a table comes from.

Mapping:

| Thesis element | Source |
|---|---|
| Table: correctness per source and class | E1 |
| Table: coverage of the MCC subset by class | `select` histogram, E2 |
| Figure: scaling per class, dispatch vs general | E2 |
| Table: paired tests per class | E2 |
| Table and figure: effort-limited answers | E3a |
| Figure: timeout overshoot | E3b |
| Cactus plot and PAR-2 table | E4 |
| Table: QR/QC by stratum | E5a, E5b |
| Table: latency | E6 |
| Appendix: triaged disagreements | adjudication |
| Threats to validity | section 13 |

## 13. Threats to validity (written into the chapter)

- **Truth for `gen` and for QR/QC targets is the agreement of two third-party tools**; a shared semantic misreading or conversion fault could affect both. Mitigations: CALIBRATE against hand answers and the MCC oracle, the MCC oracle as an independent source on `mcc-*`, and hand investigation of every conflict.
- **The MCC oracle is a consensus of tools, not a proof.**
- **Selection**: models with a known state space are the tractable ones; `mcc-large` is the (exploratory) counterweight. Median-size selection per family is a rule, not a random sample.
- **MCC nets are mostly general-class and weighted**; structural theorems barely apply, so class-specific claims rest on `gen` and `fam`, which are small, unweighted and synthetic.
- **QR and QC**: labels come from two tools agreeing, so only decidable targets are included (biased towards easier cases), and hard unreachable targets cannot be singled out because the library exposes no refinement count (section 5.5).
- **Timeouts are best effort**: the library deadline is checked only at some points; E3b characterizes this, it does not guarantee it. Parse time is outside the deadline.
- **Latency (E6)** compares integration modes, not algorithms.
- **Old, small machine**; mitigations: pinning, fixed governor, repetitions, interleaved order.
- **Reference tools are black boxes**: pinned versions, packaged by us for NixOS; conversions may be lossy; failures are counted.
- **Statistical**: timeouts are censored; effect sizes are reported with intervals; many comparisons are listed with their correction.
- **Shared authorship**: the generators and the target proposer are written by the same author as the subject; they have no authority over truth, so their errors cost instance quality, not correctness evidence. Generator output is characterised by distribution tables (class, size, verdict balance).

## 14. Coverage of claims

| Claim or element | Status |
|---|---|
| Soundness for Q0-Q6 on MCC and `gen` | Answered (E1) |
| Soundness of reachability and coverability (CEGAR) | Answered on generated nets and a restricted set of MCC models, labels from two tools (E5a, E5b); hard unreachable targets not singled out |
| Dispatch beats general on restricted classes | Answered on `gen`/`fam` (E2); on MCC only a coverage figure |
| Cost can be bounded by effort | Answered (E3a) |
| Cost can be bounded by timeout | Characterized, not guaranteed (E3b) |
| Standing against LoLA and TINA | Answered (E4, E5b) |
| Embedding latency | Answered with caveat (E6) |
| Memory | Answered (E7) |
| CEGAR refinement effort on realistic nets with hard unreachable targets | **Unanswered** (no refinement counter) |
| Unbounded nets (Q1, Q3, Q4) | **Out of scope** |
| WASM, portability to the browser | **Dropped** |
| R1-R8 self-rating, CPEE integration case study, agent study, API ergonomics | **Not part of this evaluation**; separate short sections of the chapter |
| Traceability to theory (deciding method) | Dropped from the claims |

## 15. Changes since v1 and decisions made

- Dropped: WASM; oxiz as an evaluation configuration; SMPT, verifypn, PM4Py; the naive independent oracle (BFS), class detector and invariant checker as testkit components; fixtures as a source; `native` instance format; `method` field; a pipeline stage for the pilot.
- Added: Q5, Q6, QR, QC; `mcc-large`; E3b; cache-coherence pass; CLI smoke; CALIBRATE as a gate.
- In-process execution for `gen` (decision 2026-10-22), with sharding and a CLI smoke check as compensation.
- Testkit reduction (decision 2026-10-22): truth only from the MCC oracle or from LoLA and TINA agreeing; MCC 2026 instead of 2025; models downloaded fresh for the selected subset only.
- H3b threshold intentionally open until the pilot.

## 16. Work plan and order

Ordered by dependency, with gates. Estimated 10 working days; drop order when short is listed last.

1. **Packaging gate (day 1, started)**: LoLA and TINA derivations in the petrivet flake (`nix/lola.nix`, `nix/tina.nix`, devshell `eval`). LoLA builds on darwin. To do on the NixOS box: build both, then convert one net with `ndrio` to LoLA's format and feed it to LoLA. If a tool exceeds half a day, drop it and write the fallback evaluator (section 7).
2. **Library deadline fixes (done 2026-10-22, uncommitted)**: the explicit-state fallbacks in liveness and boundedness now check the deadline, and the Commoner-Hack encoding is bounded by a check between rounds plus a Z3 interrupt watchdog. On `aslink_01_a` the overshoot at a 1 ms timeout fell from 44 s (`df`) and minutes (`live`) to about 5 ms and 9 ms. Remaining known blind spot: the structural LP in boundedness (about 200 ms on that net). The pilot still characterizes the rest.
3. **CLI** (done, uncommitted): `--timeout-ms`, `onesafe`, `stable`, `reach`, `cover`, `--target`. Open: whether dispatch should try the structural shortcut before CEGAR for `reach` and `cover`.
4. **Testkit**: generators with construction assertions, target proposer, manifest writer.
5. **`select` and `truth`**: MCC subsets from the 2026 oracle with the class histogram, fresh model downloads, truth files, CALIBRATE with the 30 hand nets and the LoLA/TINA-against-MCC check.
6. **Pilot (manual)**: timeout overshoot, family size ranges, cap and repetition counts; freeze `eval.toml`.
7. **Campaign A**, then adjudicate and triage (bugs found here go back to the library before B).
8. **Campaigns B and C**, then **D**.
9. **Analyze and export**, thesis integration, appendix of triaged disagreements.

Drop order if time runs out: `mcc-large` stratum, then E5b (MCC QR/QC), then D, then the cache-coherence pass. The core (E1, E2, E3a, E4) is not dropped.

## 17. Parameters left open (decided by the pilot, then frozen)

Family size ranges per class; wall cap and repetition rule (defaults in section 8.2); H3b threshold or distribution-only reporting; number of `mcc-small` models (about 90 by default); number of MCC models used for QR/QC (30 by default).

## Amendments

- 2026-10-22, selection rule for the MCC subsets (made before any measured run, after the first selection showed that 25 of 92 small models and a similar share of large ones could not be read by petrivet): a model counts as drawn only if petrivet imports it (connected, ordinary arcs, not colored; for `mcc-large` also within the 5 MB cap). Rejected candidates are recorded with their reason in `corpus/mcc/selected.json` and reported as coverage of the drawn sample. For `mcc-small`, per family the first parsing model in order of distance of its state-count rank from the family median is taken (pass 1, at most one per family); if that gives fewer than 90 models, a second parsing model is added per family in a fixed seeded family order (pass 2). Result: 90 models, 67 families contribute (44 with one, 23 with two), 25 families contribute nothing because none of their models parse. `mcc-large`: 5 per examination, 20 in total, no list exhausted. This replaces the rule "no filtering on whether petrivet can parse the model" of section 5.2; the unfiltered draw is still reported in the chapter as the coverage figure.
- 2026-10-22, the petrivet PNML importer now rejects arc weights other than 1 (`UnsupportedArcWeight`) instead of reading them as weight 1.

