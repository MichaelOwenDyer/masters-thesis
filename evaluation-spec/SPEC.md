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
| C3 | The caller can restrict runtime cost (RQ2, R3) | **H3a**: there are instances where `pv-poly` returns a definite, correct verdict within the cap and `pv-general` times out. **H3b**: the library `timeout` returns close to its deadline. The threshold for "close" is 200 ms (`[e3] overshoot_threshold_ms` in `eval.toml`), set provisionally and confirmed unchanged after the by-hand pilot (section 9); it is frozen before E3 runs. | E3 |
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

- Modes: `dispatch` = library default effort; `poly` = efficient effort only (may answer `unknown`); `general` = explicit-state procedure only, no structural shortcuts.
  General mode stops as early as the property allows: deadlock-freedom, 1-safety and reachability stop at the first witness (`find_until` on the reachability explorer); coverability is one pass over the Karp-Miller coverability graph; bounded uses the coverability exploration, which stops at the first omega, so an unbounded net answers `false` quickly; state space, liveness, quasi-liveness and stable need the complete reachability graph and use direct reachability exploration (the omega-checking exploration was more than 100 times slower on bounded MCC models), so on unbounded nets they end in TIMEOUT or MEMOUT. `--timeout-ms` applies in every mode.
  For `reach` and `cover`: `dispatch` = the efficient check (circuits, strongly connected state machines, trap refutation), then the library's CEGAR pipeline; `poly` = the efficient check only (`unknown` otherwise); `general` as above.
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
| `pv-inproc-parsed` | the library called from the evaluation process on an already parsed net, a fresh copy per call (the library caches liveness and deadlock results per `PetriNet` object) | latency (D) only, never wall-time comparisons |
| `pv-inproc-cached` | as above on one object per sequence: the first call computes, later calls are cache hits | latency (D), reported as a caching note, not as the headline |
| `pv-inproc-parse` | file read, PNML parse and query in each call | latency (D) only |
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
| `gen`, and QR/QC targets on `gen` and `mcc-small` | LoLA and TINA agree, and neither is inconclusive within the labelling limit (`petrivet-eval truth --label-limit-secs`, default 60 s) |
| QR/QC targets on `gen` nets that are unbounded (Q2 truth FALSE) | LoLA alone, source `lola-only`: TINA is not run there (it timed out on every unbounded calibration net and each run costs the full labelling limit), and a definite LoLA answer is the label. If TINA has a definite answer for the cell anyway (from an earlier run), the two-tool rule applies. |

If LoLA and TINA disagree, or one is inconclusive, the instance (or target) is excluded and counted.
Labels from LoLA alone are a weaker tier: they are never pooled with the two-tool labels (E1 and E5a report them as separate rows), and a petrivet disagreement with one is classified `petrivet-vs-single-tool` and triaged by hand first (for example by running TINA longer on that one cell). On an unbounded net LoLA can usually answer "reachable" (it finds a witness) but rarely "unreachable", so these labels skew toward TRUE; E5a says so beside the table. In the 20-net trial, LoLA and TINA never disagreed on the 127 units both labelled, and LoLA had no disagreement with the hand answers of the calibration set. The exclusion is decided in the TRUTH stage, before any petrivet answer is looked at, so petrivet's results cannot bias it.
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
| verdict | enum | TRUE, FALSE, COUNT(states,edges), UNKNOWN, NOT_SUPPORTED, TIMEOUT, MEMOUT, INPUT_REJECTED, CRASH, RUNNER_ERROR, SHARD_FAILED (see Outcomes below) |
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

**detail** (optional string): for CRASH one of `panic`, `abort`, `signal:<NAME>`, `exit:<code>`, `internal`; for INPUT_REJECTED the first line of the diagnostic (at most 200 characters); for RUNNER_ERROR the reason. `signal` and `exit_code` are kept as recorded.

#### Outcomes

Every non-answer settles three questions: did the tool decline, did we stop it, or did it fail?
- Answers: TRUE, FALSE, COUNT.
- Declined by the tool, no fault: UNKNOWN (ran to completion and gave up or has no applicable procedure, including its own internal timeout), NOT_SUPPORTED (query not offered for this tool or instance).
- Stopped by us: TIMEOUT (our wall-clock kill), MEMOUT (our memory limit).
- Failed, the tool did not answer and we did not stop it: INPUT_REJECTED (clean error exit with a diagnostic: the instance could not be read; for petrivet the importer errors such as a disconnected net or a weighted arc; for reference tools conversion or parse errors), CRASH (abnormal termination: a signal we did not send such as SIGSEGV, SIGABRT, SIGBUS, SIGILL, SIGFPE; exit code 101, an uncaught Rust panic; the CLI's caught panic).
- Infrastructure, counted against no tool: RUNNER_ERROR (we could not spawn the tool, disk full, output unparseable although the exit was clean, adapter bug), SHARD_FAILED (campaign A, a dead shard).

Classification, first match wins: spawn failure is RUNNER_ERROR; our wall kill is TIMEOUT; memory limit hit is MEMOUT (SIGKILL, or SIGTERM when systemd stops the cgroup scope after an OOM kill, including exit status 137 and 143 of shell wrappers, with peak RSS within 10 percent of the limit; or an out-of-memory message); a signal we did not send is CRASH; exit code 101 is CRASH(panic); the tool-specific rule for any other nonzero exit (petrivet: a JSON error line with `error_kind` panic or internal is CRASH, input is INPUT_REJECTED; otherwise INPUT_REJECTED if there is a diagnostic, CRASH(`exit:<code>`) if not); exit 0 with unreadable output is RUNNER_ERROR; otherwise the tool's verdict. Only TRUE, FALSE, COUNT and UNKNOWN are repeated.

`env.json`: CPU model, pinned cores, governor, turbo state, kernel, RAM, versions and hashes of every solver, rustc/cargo versions, git sha of the workspace, git revision of the petrivet library as locked in `Cargo.lock` (`petrivet_lib_rev`), hash of `eval.toml`, hash of the corpus, timestamp.

### 8.2 Machine and limits

- Linux box only (i7-4790K, 32 GB, NixOS). Never measure on the Mac.
- Governor `performance`, turbo off during runs, jobs pinned with `taskset` to separate physical cores, at most 3 jobs in parallel, hyper-thread siblings idle, 8 GiB memory limit per job.
- Build: `cargo build --release` from the flake; the exact binary hash is in `env.json`.
- Wall cap for campaigns B and C: 30 s per run (all sources; `[run] wall_cap_secs`). Labelling runs of `truth` use a 60 s limit (`--label-limit-secs`). Timeouts count as 2 x cap for PAR-2. Campaign A runs as CLI subprocesses with a 30 s cap per run (`[run] a_wall_cap_secs`); its wall times are not analysed.
- Repetitions (campaign B only): 5 if the first run takes under 1 s, 3 if under 10 s, otherwise 1. Timeouts are not repeated. Repetitions double as a determinism check: a verdict that differs between repetitions is flagged as a finding.
- Order: runs are shuffled with a seed and solvers are interleaved per instance, so thermal and cache drift does not favour a solver. A warm-up pass reads every instance once before measuring (cold-cache effects are a legitimate condition but not what is studied).
- Startup baseline: every solver gets a no-op run on a trivial net, once per campaign.

## 9. Campaigns

### A: correctness (subprocess, `gen`)
- Every `gen` net is run through the CLI like in B (`petrivet-eval run --campaign A`), with campaign label `A`, solvers `pv-dispatch`, `pv-general`, `pv-poly`, and the queries that apply to `gen` (Q1, Q2, Q3, Q4, QR, QC; every QR and QC target of the manifest). One repetition per job, no repetition rule, no startup baseline, no warm-up, no pinning required. Wall cap 30 s per run (`a_wall_cap_secs`), a safety cap and not a measurement. Results go to `results/runs.jsonl`; the run is resumable (existing `run_id`s are skipped) and can be split with `--shard i/n` (stable by instance hash).
- The CLI is the subject: a panic, abort, stack overflow or hang is a `CRASH` or `TIMEOUT` of one run, classified by the usual rules, and never takes other runs down. Outcomes are compared by adjudication (section 10) against the other modes and, where available, the reference tools; the summary lists per solver the instances with `CRASH` or `INPUT_REJECTED`. This replaces the in-process harness with shards, `SHARD_FAILED` and the separate CLI smoke check: the CLI's flag and verdict handling is now exercised by every run.
- Fresh process per run: no cache is shared between runs. Whether the answer to one question depends on the questions asked before on the same `PetriNet` object (the library caches boundedness, liveness and deadlock results per object) is a library-level test, the cache-coherence test in the testkit (`petrivet-testkit/tests/cache_coherence.rs`): about 200 generated nets of all classes and size buckets, the six questions asked on a fresh object each and on one object in seeded permutations, answers (including `None` against `Some`) must be identical, and `Some` answers must agree with an explicit-state computation where the reachability graph is small.
- Not measured here: time and memory.

### B: performance and comparison (subprocess, `fam`, `mcc-small`, `mcc-large`)
- Solvers: `pv-dispatch`, `pv-general`, `pv-poly`, `lola`, `tina`. Queries per section 3.
- Expected run count: about 600 (instance, query) pairs x 5 solvers x about 3 repetitions on average; upper bound of runtime if 25 percent of solver-runs hit the cap is about 4 hours with 3 parallel jobs. Recompute after the pilot.

### C: timeout behaviour (E3b)
- Instances: cells (instance, query, target) of B whose median `pv-dispatch` wall time is more than 2 x the largest t, so the deadline actually triggers; a B TIMEOUT counts as slow, CRASH, MEMOUT, INPUT_REJECTED, RUNNER_ERROR and NOT_SUPPORTED cells are left out. `[run] c_timeouts_ms` lists the t.
- `pv-dispatch` with `--timeout-ms t`, t in {10, 100, 1000}, one run per cell (3 repetitions where the first run is under 1 s and gave TRUE, FALSE, COUNT or UNKNOWN). External wall cap `[run] wall_cap_secs`; no startup baseline or warm-up. The `run_id` hashes the solver as `pv-dispatch@t<t>` (the record's `solver_id` stays `pv-dispatch`), the record has `timeout_ms`.
- Measures: `overshoot = analyze_ms - t` (absolute, parse excluded and stated; stored in `note` as `overshoot_ms=<value>` and recomputable from `analyze_ms` and `timeout_ms`), verdict (UNKNOWN vs definite), external wall as a sanity check.
- Pilot: `run --campaign C --instances id1,id2 --results-subdir pilot` runs chosen instances without B results.
- The by-hand pilot comes first (below), then the threshold is frozen. C itself is then part of the pipeline.

### D: latency (E6) -- DROPPED, see the amendment of 2026-10-23
- Instances: 20 small `gen` nets per class (about 120), queries Q1 and Q3.
- Command: `petrivet-eval latency <gen manifest>`, output `results/latency.jsonl`. "Small" means `size_bucket` 2-5, 20 nets per class chosen by seeded hash.
- Variants: `pv-inproc-parsed` (fresh copy of the parsed net per call; cold first call and steady state reported separately; the headline), `pv-inproc-cached` (one object per sequence, cache hits after the first call), `pv-inproc-parse` (file read, parse and query), `pv-dispatch` CLI subprocess, `lola`, `tina`; each query repeated in a sequence of 200 (a sequence stops at its first call that gives no answer); startup baselines (no-op spawn and `petrivet --version`) reported and subtracted from subprocess latencies.
- Expectation: microseconds in-process, milliseconds for subprocess tools. The result mostly reflects the integration mode (process start, Z3 load), not the algorithm. The chapter says exactly that.

### The pilot (manual, one-off, not a pipeline stage)
Before freezing `eval.toml`: run C on about 10 hand-picked instances, look at the overshoot, and if there are large outliers run `perf` on them once to see which phase holds the thread (LP solve, siphon enumeration, exploration between checks, parse).
The pilot also fixes the family size ranges, the cap and repetition counts. Its data is stored in `results/pilot/`, labelled exploratory, and not used in any reported test.
Output of the pilot: the H3b threshold (or a decision to report only the distribution), written into `eval.toml` and committed before any campaign runs.

## 10. Adjudication (`results/adjudication.jsonl`)

One line per run joined with truth. Outcomes: AGREE, DISAGREE, UNKNOWN, TIMEOUT, MEMOUT, INPUT_REJECTED, CRASH, RUNNER_ERROR, NOT_SUPPORTED, NO_TRUTH (no truth for the cell), OUT_OF_SCOPE (for example Q1, Q3, Q4 on a net known to be unbounded).
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
| E1b | Cache coherence | the testkit test `cache_coherence` (answers independent of question order and of the `PetriNet` object); count of differences, expected zero |
| E2 | Dispatch ablation | `fam` only (`gen` nets are too small for timing): per instance median time of `pv-dispatch` vs `pv-general`, paired; Wilcoxon signed-rank per class, Holm-corrected; Hodges-Lehmann estimate and median ratio with IQR; instances where either times out are censored at the cap, with the count reported and a sensitivity run excluding them; frontier = largest size solved within the cap; peak RSS compared likewise. On `mcc-small`: only the share of instances where dispatch differs from general (class detection and `pv-poly` definite) |
| E3a | Cost control by effort | share of `pv-poly` runs that are definite and correct, UNKNOWN share, instances where `pv-poly` answers and `pv-general` times out (`fam`, `mcc-large`; `mcc-large` flagged exploratory) |
| E3b | Cost control by timeout | distribution of overshoot per instance family; share within the (frozen) threshold; the instances responsible for the largest overshoots (`e3b_top_overshoot.csv`; the phase is found by the manual `perf` follow-up, the CLI reports none); share UNKNOWN vs definite per t |
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
- Campaign A as CLI subprocesses instead of the in-process harness (decision 2026-10-23); the cache-coherence check is a testkit test.
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
7. **Campaign A** (CLI subprocess runs on `gen`; cache-coherence test in the testkit), then adjudicate and triage (bugs found here go back to the library before B).
8. **Campaigns B and C**, then **D**.
9. **Analyze and export**, thesis integration, appendix of triaged disagreements.

Drop order if time runs out: `mcc-large` stratum, then E5b (MCC QR/QC), then D, then the cache-coherence pass. The core (E1, E2, E3a, E4) is not dropped.

## 17. Parameters left open (decided by the pilot, then frozen)

Family size ranges per class; wall cap and repetition rule (defaults in section 8.2); H3b threshold or distribution-only reporting; number of `mcc-small` models (about 90 by default); number of MCC models used for QR/QC (30 by default).

## Amendments

- 2026-10-22, selection rule for the MCC subsets (made before any measured run, after the first selection showed that 25 of 92 small models and a similar share of large ones could not be read by petrivet): a model counts as drawn only if petrivet imports it (connected, ordinary arcs, not colored; for `mcc-large` also within the 5 MB cap). Rejected candidates are recorded with their reason in `corpus/mcc/selected.json` and reported as coverage of the drawn sample. For `mcc-small`, per family the first parsing model in order of distance of its state-count rank from the family median is taken (pass 1, at most one per family); if that gives fewer than 90 models, a second parsing model is added per family in a fixed seeded family order (pass 2). Result: 90 models, 67 families contribute (44 with one, 23 with two), 25 families contribute nothing because none of their models parse. `mcc-large`: 5 per examination, 20 in total, no list exhausted. This replaces the rule "no filtering on whether petrivet can parse the model" of section 5.2; the unfiltered draw is still reported in the chapter as the coverage figure.
- 2026-10-22, the petrivet PNML importer now rejects arc weights other than 1 (`UnsupportedArcWeight`) instead of reading them as weight 1.
- 2026-10-23, campaigns C and D and the analysis package are implemented (`run --campaign C`, `latency`, `evaluation/analysis`). Analysis hashes (`eval.toml`, results files) are sha256; `env.json` uses blake3. The provisional H3b threshold is 200 ms.
- 2026-10-23, `truth` default labelling limit 60 s (was 600 s: the first trial spent 60 of 128 run-minutes on six time-outs); TINA is skipped for QR/QC on unbounded `gen` nets and a definite LoLA answer there is labelled `lola-only` (section 7).
- 2026-10-23, campaign D (latency, E6) and the latency half of H4 are dropped: the expected difference between in-process and subprocess calls is negligible against the cost of the analysis questions themselves, and time is short. The `latency` subcommand and the E6 analysis stay in the code, unused. The evaluation chapter states as a non-claim that it does not measure in-process overhead; every tool is measured as a separate process and the start-up time of each tool is reported on a trivial net. Research questions of the chapter: RQ2 dispatching, RQ3 cost control, RQ4 correctness, RQ5 comparison in speed and memory.
- 2026-10-24, campaign A found a soundness bug in the library: the CEGAR analysis proved 9 reachable QR/QC cells (8 with truth, 1 without) unreachable or uncoverable on the strength of Wimmel and Wolf increment constraints, which steer the search and are not facts about the net. The library (ca1dd34) now treats a refutation that rests on one as inconclusive. The 5,315 `pv-dispatch` QR/QC FALSE rows of campaign A were removed and rerun at the fixed revision; the other rows are unaffected (CEGAR is used only for QR/QC). After the rerun: 0 disagreements, 9 `pv-dispatch` UNKNOWN on QR/QC. The cache-coherence test passes (209 nets, no differences). The pilot for C (10 instances, 20 cells, `results/pilot`) shows overshoot medians of 5, 14 and 40 ms for t = 10, 100, 1000 ms and 4 of 20 cells above 200 ms at t = 1000 ms (causes: a stretch of about 100 to 200 ms without a deadline check in Q3 on nets of several thousand places; exploration checks that get costlier with the state set on Q1). The H3b threshold stays 200 ms; the analysis reports the distribution and the share of cells above it.
- 2026-10-24, campaign B (23,206 first-pass runs) and its adjudication found three harness faults and one library fault, all fixed and the affected cells rerun at the fixed revisions; and two adapter limitations that stay. (1) *Argument limit*: the LoLA and TINA scripts for Q3, Q4 and Q6 were passed as one `sh -c` argument, which exceeds the kernel's limit of 128 KiB per argument on nets of several thousand transitions (30 runs, 9 nets: RUNNER_ERROR "Argument list too long"). The scripts now go to `sh -s` on standard input and every tool call in them reads `/dev/null`. (2) *Stack overflow in the library*: the `pv-general` baseline aborted with a stack overflow on Q3 and Q4 (24 runs, 12 nets of 9 to 161 places). Cause: the liveness levels of the reachability graph used petgraph's `tarjan_scc`, which is recursive, so a path of about 90,000 to 110,000 states overflowed the 8 MB stack; the process is aborted, not an error. This is a defect of the library and not only of the baseline. The library now uses the iterative `kosaraju_scc` (regression test with a 200,001-state chain); the nets complete after the fix (about 0.2 s for the two that were timed). (3) *Memory kills read as crashes*: five LoLA Q3 runs ended in SIGTERM after 11 to 18 s and were recorded as CRASH with the shell's peak RSS of 59 to 145 MB, although the other solvers also ran out of memory on the same nets. The runner now starts the cgroup scope with `OOMPolicy=continue`; after the rerun the five runs are MEMOUT (exit status 137, peak RSS 8.4 GB). A small stand-alone test (`tail /dev/zero` under a 64 MB scope) gave status 137 with and without the property, so it does not show that the property is what changed the outcome; the evidence is the rerun. (4) *Wrong verdicts from truncated output* (the only fault that produced wrong answers): the LoLA script for Q3 and Q4 echoed the full JSON of every sub-run, which exceeds the 1 MiB output cap on nets of about 2,000 transitions; the final `PETRIVET_ANSWER` line was lost and the adapter read the last complete JSON line, a sub-run result, as the answer of the query. 28 runs of 18 cells were TRUE this way, two of them contradicted the construction truth (`ac_cycle_no_low_token`, size 1024, Q3 and Q4). The script now echoes only the sub-run that ends it, and several JSON lines without an answer line are a RUNNER_ERROR. The 28 rows were removed and rerun: that net is now FALSE for both queries, the other 16 cells kept TRUE, no repetition differs. Rows removed in total: 59 (faults 1 to 3) and 28 (fault 4). Result after the reruns: 58,430 runs adjudicated (A 52,590, B first runs 5,840), 0 disagreements, 0 crashes, 0 findings without truth, 0 repetitions that differ. Limitations that stay and are reported as such in the chapter: (a) the LoLA adapter names places by their PNML id while `ndrio` writes their name; for `Medical-PT-02` 13 places have an id that differs from the name, so the Q5 formula names places that do not exist in the converted net (INPUT_REJECTED, 1 run; it is the only such net of the 256 in B); (b) the LoLA Q5 formula is one argument and exceeds the 128 KiB limit on five nets of 7,000 to 16,000 places (RUNNER_ERROR, 5 runs). Not faults: `pv-dispatch` returns NOT_SUPPORTED for Q0 and Q6 by design (the library has no dispatching procedure for them; 180 runs, reported as "no procedure", not as inconclusive); TINA gives UNKNOWN on Q1, Q3 and Q4 of 12 unbounded nets (180 runs, its report says "not bounded"). The stale reproducers of campaign A and of fault 4 were removed from `results/reproducers`.
- 2026-10-24, campaign C ran (2,091 runs, 305 cells of 108 instances at t = 10, 100, 1000 ms; cells are the `pv-dispatch` cells of B with median wall time above 2,000 ms, excluding MEMOUT and NOT_SUPPORTED; QR and QC contribute no cells because the `fam` and `mcc` manifests carry no targets). No repetition differs from repetition 0. Overshoot (analysis time minus t): medians 3.7, 15.0 and 52.8 ms; share of runs above the 200 ms threshold 1.9 %, 1.1 % and 13.9 %; maxima 9.5, 9.4 and 8.5 s. H3b is reported as the distribution plus named blind spots, not as a pass or a fail: the library timeout returns close to its deadline for the typical cell, and not in a tail. The tail has one structural cause found by reading the code: `refine_bounds` (`api/system/boundedness.rs`) checks the deadline only between stages, and the efficient-subclass step and the structural LP receive no deadline, so a stage that is slow on a net runs to completion. Affected: Q5 (it is `is_bounded().k(1)`, the same procedure as Q2) on `FamilyReunion-PT-L00050M0005C002P002G001` and both `ServersAndClients` nets (12k to 17k places), where the run was ended by the 30 s cap at every t, and on `DLCflexbar-PT-6a` (about 5 s over); Q2 on `efc_forkjoin` and `efc_forkjoin_leaky` of size 1,024 (3,072 places), which answer TRUE after about 9.5 s even at t = 10 ms. Which stage is slow on each net was not profiled. Two further causes seen in the data: Q3 and Q4 on `RERS2020-PT-pb101` (149k transitions) overshoot by about 1.0 to 1.3 s at t = 10 and 100 ms, and Q1 on large state sets overshoots by about 400 to 500 ms at t = 1000 ms because the exploration checks get costlier with the state set (as in the pilot). Decision: the library is not changed for this; the chapter reports the tail and its causes. A fix (pass the deadline into the two stages) is future work.
- 2026-10-24, the peak memory of campaign B is not usable and is measured again (campaign M). Evidence: for runs under 300 ms the median `peak_rss_kib` of B rises from 93 MiB to 143.5 MiB over the course of the campaign and its minimum equals its median, 91.5 % of all B values lie below 160 MiB, and every tool, LoLA and TINA included, shows the same value on the calibration nets; the start-up baseline is 29 MiB and campaign A's runs of the same tools are 21 to 35 MiB. The measure is `ru_maxrss` from `wait4` on the child of the runner, and a child's figure contains what it inherited from the runner when it was started; the runner's own footprint grew over the campaign (it holds the manifests and read all 164 MB of instance files at warm-up). This explanation fits the data but was not tested separately. Consequences: memory-out classification is unaffected (it rests on the 8 GiB limit), values far above the floor are real, and everything else in the memory columns of B is not (E7, the memory comparison inside E2). Campaign M repeats the cells of B that gave an answer (TRUE, FALSE, COUNT or UNKNOWN in B; 4,688 jobs, one run each, the 1,152 other cells are left out: a time-out is killed at the cap together with the wrapper and leaves no memory value, a memory-out keeps B's value, the limit) with the tool started through a small wrapper (`petrivet-eval rss --out FILE -- tool args`, `measure.rs`) that waits for the tool itself, writes its peak memory and ends the way the tool ended, so exit codes, signals and memory-out detection are unchanged. M's times are not analysed. The analysis replaces `peak_rss_kib` of every B record by M's value for the same cell (empty where M has none, B's value for a memory-out) and checks that M and B gave the same verdicts. Limitation: a run killed at the wall cap has no memory value.
- 2026-10-24, the automatic choice (`pv-dispatch`) is slower than plain exploration on the benchmark models, the cause was found in the library, fixed, and `pv-dispatch` was run again in A, B, C and M. *Finding* (campaign B, `mcc-small`, first runs): on liveness (Q3) `pv-dispatch` answered 47 of 90 cells and hit the 30 s cap on 41, while `pv-general` answered 88; where both answered, `pv-dispatch` took a median 5.0 times as long on Q3, 6.0 times on Q4 and 1.95 times on Q1 (90th percentile about 170 times on Q3 and Q4); on the generated families the ratio was about 1.0. *Cause* (two stages, each timed in isolation on a scratch copy): (1) for nets without an efficient procedure the fallback builds the coverability graph, and the Karp-Miller acceleration (`omega_accelerate`, with the library's own `TODO: Find a more efficient algorithm`) visited, for every fired edge, every graph ancestor of the source with a fresh bitset, which is O(edges x nodes); on `SmallOperatingSystem-PT-MT0016DC0008` (16,587 markings) the coverability graph took 51 s and on `ShieldRVt-PT-003A` (8,193 markings) 36 s, against 12 ms for the reachability graph with the same number of markings. (2) For free-choice and asymmetric-choice nets the Commoner-Hack criterion: on `IOTPpurchase-PT-C01M01P01D01` (204 markings) the efficient procedures answered "none" in 9 microseconds and the explicit graph took 1.7 ms, yet the full dispatch took 32 to 36 s; this was attributed to the Commoner-Hack criterion by elimination, as it is the only other stage on that path. *Fix* (stage 1 only, library commit after 71ececd4): acceleration walks only the path of the exploration tree from the initial marking (every node on it is a real predecessor, so it is sound and is the classical check); once an ω has appeared, i.e. the net is unbounded, it uses the earlier full-ancestor traversal. A bounded net never produces an ω and so never takes the expensive path. Tree-path acceleration alone was tried first and rejected: on unbounded nets it built graphs up to 19 times larger (median 1.3 times). Evidence for the hybrid: on 247 nets (100 unbounded generated, all 146 `fam`, 20 MCC) with 5 s each, 186 finished with both versions and all 186 graphs (nodes and edges) are identical, ω present or absent is the same, and the maximal markings (coverability set; graphs up to 3,000 nodes) are identical on 179 of 179; no net that the old version finished timed out, 6 that timed out now finish; all library tests pass; a regression test (ring of three places, 250 tokens, 31,626 markings) takes 0.13 s with the fix and 40.5 s without. This is a differential test against the old version, not a proof. *Rerun*: the 24,487 earlier `pv-dispatch` rows (A 17,530, B 4,089, C 2,091, M 777) were archived to `results/archive/pv-dispatch-before-omega-fix.jsonl.gz` and removed; only `pv-dispatch` was run again (A, B, C, M); the other solvers are unaffected (`pv-poly` uses efficient procedures only, `pv-general` is the CLI's own exploration). *Result*: A, 17,530 cells: 17,526 unchanged, 4 time-outs became memory-outs. B, first runs: 1,096 unchanged, 96 time-outs now have an answer (74 TRUE, 22 FALSE, mostly `mcc-small` liveness, quasi-liveness and 1-safety on general and asymmetric-choice nets), 12 time-outs became memory-outs (the explorer now fills memory before the cap); among cells with a definite answer both before and after, 0 differ. Adjudication: 0 disagreements, 0 crashes; cache coherence: 0 differences on 209 nets; M against B: 1 verdict differs (a cell at the wall cap). On the MCC models (paired, both answered) the ratio of `pv-dispatch` to `pv-general` time went from 5.0 to 2.4 (Q3, n 47 to 82), 6.0 to 2.4 (Q4, n 48 to 82) and 1.13 to 1.09 (Q5, n 69 to 91), and the 90th percentile from about 170 to 8 on Q3 and Q4, from 36 to 4.4 on Q5; Q1 is unchanged (1.97 to 1.92, 90th percentile 70). Still open and reported as such: cells where `pv-dispatch` fails and plain exploration answers, 27 on `mcc-small` (deadlock-freedom on general nets 9; liveness, quasi-liveness and 1-safety on general and asymmetric-choice nets 3 each) and 19 on `fam` (mostly asymmetric-choice nets: the Commoner-Hack stage, which the fix does not touch); the slow deadlock-freedom (Q1) cause was not investigated; 7 `mcc-small` cells where `pv-dispatch` runs out of memory while plain exploration answers were not investigated. Campaign C after the rerun: 1,438 runs (2,091 before, because fewer cells are slow enough to qualify); overshoot medians 3.5, 10.5 and 46.5 ms at t = 10, 100, 1000 ms (3.7, 15.0 and 52.8 before); the maxima (about 9.6, 9.6 and 8.6 s) and the three Q5 time-outs are unchanged, because the tail comes from stages that never check the deadline. This replaces the C figures of the amendment above. *Caveat*: the `pv-dispatch` rows of B, C and M were measured in a later session than the other solvers' B rows and without them running next to it; the machine settings are the same. The earlier results stay available in the archive for a before and after statement in the chapter.
