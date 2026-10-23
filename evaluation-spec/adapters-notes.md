# Adapter notes: LoLA 2.0 and TINA 4.0.0

Accessed 2026-10-22. Written from the tools' own documentation only; nothing below was executed
(the machine was aarch64-darwin, both TINA binaries are x86-64 Linux ELF, and LoLA was only compile-tested).
Anything not confirmed from documentation or source is marked UNVERIFIED and must be settled by CALIBRATE on the NixOS box.

## Sources

| What | URL / path | Notes |
|---|---|---|
| LoLA 2.0 source tarball | https://theo.informatik.uni-rostock.de/storages/uni-rostock/Alle_IEF/Inf_THEO/images/tools_daten/lola-2.0.tar.gz | linked from https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/ . sha256 `ma+oO44SFkb4JGchR3cUzTBnv9vWPaGvKkdMJoL563E=` (SRI). The old URL http://service-technology.org/files/lola/lola-2.0.tar.gz now returns a parked-domain HTML page, not the tarball. Dated 2016-04-20 inside the archive. Licence: AGPL-3.0 (COPYING; file headers say "any later version"). |
| LoLA manual | `doc/lola.texi`, `doc/lola.man`, `doc/outputs/*.txt` inside the tarball | all LoLA statements below cite this unless stated; `tests/testsuite.at` is cited where the manual is silent |
| TINA 4.0.0 binaries | https://projects.laas.fr/tina/download.php (page says "Release 4.0.0", last updated 2026-07-05), https://projects.laas.fr/tina/binaries/tina-4.0.0-amd64-linux.tgz | `-large-` variant: 64-bit marking encoding instead of 32-bit |
| TINA manuals | `tina-4.0.0/doc/txt/{ndrio,tina,sift,scan,selt,struct,mcc,formats}.txt` inside the tarball | all TINA statements below cite these |
| TINA licence | none found | no licence file in the tarball, no licence statement on download.php, home page, or apl.php. Treated as unfree / redistribution terms unknown. |

Update after the first draft: the Nix build of LoLA 2.0 succeeded on aarch64-darwin, and `lola examples/phils/phils10.lola --check=full --stubborn=off --json=/dev/stdout --quiet` returned `"result": false`, `"states": 59048`, `"edges": 393650` (matches the manual). JSON is emitted as a single line; exit status 0. This confirms Q0 and the JSON field names for LoLA only.

## (a) ndrio: formats and PNML to LoLA

From `ndrio.txt`:

- Reads (input flags `-NET | -NDR | -PNML | -TPN`, or by extension): net, ndr, tpn, pnml.
- Writes (output flags `-net | -ndr | -pnml | -tpn | -pnt | -romeo | -lola | -nupn`): net, ndr, tpn, pnml, pnt (INA), romeo, lola, nupn.
- The conversion table in the manual lists `pnml -> net, ndr, tpn, pnml, pnt, romeo, lola, nupn`, so PNML to LoLA is supported directly.
- The manual calls the lola and romeo export "preliminary" and describes the lola format as "unstructured".

Command:

```
ndrio -PNML -lola model.pnml model.lola
# equivalently, by extension:
ndrio model.pnml model.lola
# stdin/stdout:
ndrio -PNML -lola < model.pnml > model.lola
```

Caveats stated by the manual:

- "the user should make sure the place and transition names used in the input file are legal romeo or lola names; ndrio does not attempt to convert them (consider passing options -r -d, in addition)". LoLA identifiers must not contain `, ; : ( ) { }` or whitespace (lola.texi, regex for IDENTIFIER). `-ren` (alias `-r`) renames to `p1..`, `t1..` and keeps the old names as labels; `-d` drops labels. Renaming breaks name-based targets for QR/QC, so the adapter must keep its own old-to-new map if it uses `-r`.
- Export fails with "can't export to lola: net with priorities" and "...net with special arcs" (strings in the `ndrio` binary; read, not run). Not relevant to plain P/T nets.
- For PNML with Place/Transition semantics, `ndrio` reads the default "tpn" profile of TINA and, presumably, standard P/T PNML. UNVERIFIED: behaviour on MCC-style PNML (ids equal to names, possibly with `<name><text>`), on arc weights and initial markings in `<initialMarking>`, and on high-level (colored) PNML, which ndrio does not handle. For colored MCC models, TINA ships `mcc` ("High-Level Petri nets unfolder": `mcc lola -i model-COL.pnml -o ...`, `mcc pnml`, `mcc tina`, `mcc smpt`; mcc.txt). Whether `mcc lola` accepts already-P/T PNML is UNVERIFIED.
- UNVERIFIED: that LoLA 2.0 accepts the `.lola` file that ndrio emits. The `ndrio` binary contains the keywords PLACE, MARKING, TRANSITION, CONSUME, PRODUCE (strings), which match the LoLA 2 grammar (lola.texi section "Petri nets"), but exact syntax (separators, zero-token places, SAFE) was not seen. Check on the first net (SPEC.md already says so).
- The 2016 LoLA parser reads `PLACE`/`MARKING`/`TRANSITION ... CONSUME ... PRODUCE`; it is documented as compatible with "LoLA 1.x" format (section Compatibility, not read in detail).

## (b) Query commands

Notation: `N.lola` is the converted net, `N.pnml`/`N.net` the TINA input. LoLA prints its human-readable result lines on stderr (tests/testsuite.at greps `stderr` for `lola:\s*result: no`); with `--json` the result is on stdout (or a file). `analysis.result` is true iff the formula holds.

### LoLA 2.0

LoLA has no whole-net operators for boundedness, liveness, quasi-liveness; the manual recommends per-place/per-transition runs (lola.texi, sections "Boundedness of the whole net", "Liveness of the net", "Quasi-liveness") and a conjunction/disjunction in one formula is the single-run alternative. Both are shown. Polarity: `--formula="EF phi"` prints `result: yes` iff some reachable marking satisfies phi; `AG`, `IMPOSSIBLE` print yes iff it holds everywhere.

Common flags for decision runs: `--quiet --json` (add `--timelimit=SECONDS`, `--markinglimit=N`; default is unlimited). Default search is depth-first with stubborn sets on for `--check=modelchecking`.

| Query | Command | Reading |
|---|---|---|
| Q0 | `lola N.lola --check=full --stubborn=off --json` | `analysis.stats.states` and `analysis.stats.edges` (text: `lola: 59048 markings, 393650 edges`, phils10 example, doc/outputs/output2.txt and tests/testsuite.at:2033). Manual says stubborn sets are switched off for `--check=full` anyway; the test suite passes `--stubborn=off` explicitly. Do not add `--symmetry`. Only for bounded nets. |
| Q1 | `lola N.lola --formula="EF DEADLOCK" --quiet --json` | result true = a deadlock exists = not deadlock-free (invert for the spec's Q1). Manual section "Deadlocks". |
| Q2 | `lola N.lola --search=cover --encoder=full --quiet --json --formula="IMPOSSIBLE (p1 = oo OR p2 = oo OR ... )"` listing all places | result true = bounded. Basis: the manual's per-place form is `--search=cover --encoder=full --formula="AG p < oo"`; the whole-net single-formula form is the shape of `examples/planner/planner_boundedness.formula` in the tarball (`IMPOSSIBLE (BeamerUsed = oo OR ...)`). Without `--search=cover` LoLA only warns "omega markings used without --search=cover" (tests/testsuite.at:818-822). UNVERIFIED: semantics when the coverability graph gives "unknown" (JSON result null), and that the one big OR is as fast as per-place runs. Note that the manual's own worked example writes `AG p <= oo` in the command but `< oo` in the text; use the shipped example's `= oo` form and confirm in CALIBRATE. |
| Q3 | per transition t: `lola N.lola --formula="AGEF FIREABLE(t)" --quiet --json`; net is live iff all return true. Single run: `--formula="AG ( (EF FIREABLE(t1)) AND (EF FIREABLE(t2)) AND ... )"` | Manual: "Liveness of a transition" (current release uses the generic CTL algorithm), "Liveness of the net". This is L4 liveness for each transition over R (AG EF fireable). UNVERIFIED: that the combined formula parses as written and its performance; the per-transition loop is the documented route. |
| Q4 | per transition t: `lola N.lola --formula="EF FIREABLE(t)" --quiet --json`; quasi-live iff all true. Single run: no single-run reachability form (conjunction of EFs is not a simple reachability query) | Manual "Dead transition": dead is `AG NOT FIREABLE(t)`; quasi-live is its negation for every t. |
| Q5 | `lola N.lola --formula="AG (p1 <= 1 AND p2 <= 1 AND ...)" --quiet --json` or `--formula="IMPOSSIBLE (p1 > 1 OR p2 > 1 OR ...)"` | true = 1-safe. Manual "k-boundedness of a place" (`AG p <= k`, recognised as a reachability query). Whole-net via conjunction: manual recommends per place for efficiency but the conjunction is logically the same. The `SAFE` annotation in the net file is not enforced (lola.texi). |
| Q6 | per place p with initial tokens m0(p): `--formula="AG p = m0(p)"`; stable place exists iff at least one returns true. m0 comes from the net's MARKING section | UNVERIFIED but follows directly from the formula grammar (`term = term` atomic proposition, lola.texi / grammars/formula.ebnf). No dedicated LoLA query for this MCC examination was found in the manual. Consider `--formula="EF p != m0(p)"` equivalently (result false = stable). |
| QR | `--formula="EF (p1 = a1 AND p2 = a2 AND ... AND pn = an)"` with every place of the net listed (places at 0 included, otherwise it is a partial-marking query, which is a different question) | true = reachable. Manual "Reachability". |
| QC | `--formula="EF (pi >= ai AND ...)"` over places with ai > 0 | true = coverable. For unbounded nets LoLA still answers from the reachability search; UNVERIFIED whether it terminates, add `--timelimit`. |

LoLA also has `--stateequation` (structural necessary-condition check, tests/testsuite.at:983-997) and `--stubborn`/`--symmetry`/`--cycle` reductions; none are needed for adapters and each is a possible source of disagreement, so keep defaults.

### TINA 4.0.0

Facts that follow from the manuals:

- `tina` builds a state-space abstraction and, per the `-d` flag description, prints a five-line digest with boundedness, liveness, reversibility, counts, `dead` (deadlock states and unfireable transitions) and `live` counts (tina.txt, "Other flags"). "unfireable transitions" is TINA's `dead` transition count: a transition is dead if it never fires from the initial marking, i.e. not quasi-live. A transition is "live" if it fires in all pending (terminal) strongly connected components of the graph, which is L4 liveness on a finite graph.
- Default for untimed nets is `tina -R -s 1 -c 0 -t 0 -b 0 -m 0 -v` (marking graph; `-s 1` applies a sufficient unboundedness test that stops construction).
- `-C` builds the Karp-Miller coverability graph; the binary's strings contain "bounded", "not bounded", "unbounded places include:", "live", "not live", "possibly live", "reversible", "LIVENESS ANALYSIS", "ANALYSIS COMPLETED" and `dead transition(s):`. The exact printed layout is UNVERIFIED; read it from a first run.
- `sift` is a faster specialisation of `tina` (less options). `-f form` stops exploration if some state violates `form` (modality-free selt formula, plus atom `safe`/`L.safe` meaning every place has at most one token); `-dead` stops at the first deadlock (shorthand for `-f "-L.dead"`). `sift -k`/profile flags write `.ktz` that `scan`, `selt`, `muse` read. `scan` counts states in a ktz obeying a formula and, with `-b`, prints TRUE/FALSE. With `-r form -b` TRUE means a state satisfying form exists (EF form); with `-n form -b` TRUE means no counted state (AG form) (scan.txt).
- Profiles for `sift` ktz output: `-rsf` (reachability set + firable transitions: reachability, deadlock, quasi-liveness), `-rsd` (reachability + deadlocks), `-rs` (reachability only) (sift.txt).

| Query | Command | Reading |
|---|---|---|
| Q0 | `tina -R -q N.pnml` (or `sift -R N.pnml` default `-q`) | The digest/summary reports counts of states and transitions of the marking graph. Exact line format UNVERIFIED. Only for bounded nets: use `-c LIMIT` or `-t SECONDS` to bound the run. |
| Q1 | `sift -R -dead -q N.pnml` is the on-the-fly check (stops at first deadlock); alternatively `tina -R -q N.pnml` and read the digest `dead` state count; or `sift -R -rsd -k N.pnml N.ktz` then `scan -dead N.ktz` (count of deadlocked states, nonzero = deadlock exists). Also `tina -R -en 1` checks absence of deadlock on the fly (tina.txt, `-en n`). | Which output line signals a found deadlock for `sift -dead` is UNVERIFIED. |
| Q2 | `tina -C -q N.pnml` | Coverability graph; digest line reports bounded / not bounded (strings in binary). The manual notes boundedness is undecidable with time constraints, inhibitor or stopwatch arcs, which plain P/T nets do not have. |
| Q3 | `tina -R -q N.pnml` and read the live/not live line | Valid only if the marking graph is finite (bounded net); tina's "live" is defined via terminal SCCs (above). For unbounded nets, TINA prints "not applicable, net is unbounded" (string in binary) or "possibly live". Treat as not supported there (spec excludes unbounded nets for Q3). |
| Q4 | `tina -R -q N.pnml`, quasi-live iff zero unfireable ("dead") transitions in the digest; or `tina -Vq N.pnml` ("-Vq stops as soon as all transitions have been fired ...; it checks quasi-liveness", tina.txt) | `-Vq` is the documented quasi-liveness mode and can stop early. Output line UNVERIFIED. |
| Q5 | `sift -R -f safe -q N.pnml` | `-f safe` stops when some state violates safeness; how violation is reported is UNVERIFIED. `struct -safe` only evaluates a sufficient structural condition (struct.txt), so a negative answer from it is not a refutation. A possible alternative is `sift -R -b 1`: "Stops if the marking of some place exceeds n" (sift.txt). |
| Q6 | `sift -R -k N.pnml N.ktz` (default profile `-kts`), then for each place p with initial tokens m0: `scan N.ktz -n "p = m0" -b` (TRUE iff p equals m0 in every state); stable place exists iff any is TRUE | Formula syntax from selt.txt: atoms are place names, `p1 = 2`, `p1+p2 > p3`, `/\`, `\/`, `-` (negation); `p1` alone means `p1 > 0`. UNVERIFIED: the `-b` TRUE/FALSE strings and that `p = 0` parses (selt.txt shows `-p1` as equal to `p1 = 0`). Bounded nets only. |
| QR | `sift -R -rs -k N.pnml N.ktz`, then `scan N.ktz -r "p1 = a1 /\ p2 = a2 /\ ..." -b` listing every place (use `-p` for zero places, i.e. `-p3` for p3 = 0) | TRUE iff reachable. Full-marking equality needs all places mentioned. UNVERIFIED: exact output strings; bounded nets only for a complete answer. A target found early could be checked on the fly with `sift -f "-(target)"` (stop when the target is reached), not confirmed. |
| QC | `scan N.ktz -r "p1 >= a1 /\ p2 >= a2 ..." -b` | TRUE iff some reachable marking covers the target. For unbounded nets use a coverability ktz: `tina -C N.pnml N.ktz`; UNVERIFIED whether scan/selt handle omega weights in formulas (the ktz format "natively supports weighted state properties, even for infinite weights", tina.txt). |

## (c) Timing and exit status

### LoLA 2.0

- Exit status: `exit 0` (EXIT_NORMAL) for any completed analysis, whether the answer is yes or no; `1` (EXIT_ERROR) for errors (syntax errors, file errors; error codes #01 to #04 are printed); `2` (EXIT_TERMINATION) for premature termination (src/Core/Dimensions.h:195-197; lola.texi, "Error messages": "LoLA exits with exit code 1"). So the verdict is not in the exit code. The test suite shows `exit 1` also for invalid input and some unsupported store options.
- Machine-readable result: `--json` (stdout or `--json=FILE`). Entries: `analysis.result` (true/false; missing when no result was found, e.g. a limit was hit; null when unknown), `analysis.stats.states`, `analysis.stats.edges`, `analysis.type`, `analysis.formula.type`, `call.error`, `call.signal`, `limits.time`, `limits.markings`, `net.places`, `net.transitions`, `store.*` (lola.texi, "JSON", `doc/outputs/output13.txt`). `--jsoninclude=path` / `state` adds witness path / witness marking. `--quiet` silences the text log.
- Timing: the JSON has no runtime field (checked in the manual and the example output). Use external wall clock and peak RSS (e.g. `/usr/bin/time -v` or the harness's own measurement). `--timelimit=SECONDS` and `--markinglimit=N` make LoLA stop itself; on a limit the JSON `analysis.result` is missing, which distinguishes TIMEOUT from yes/no. Behaviour on limit hit (exit code and exact text) is UNVERIFIED.
- Example from the manual: `lola phils10.lola --formula="EF DEADLOCK" --quiet --json | jq ".analysis.result, .analysis.stats.states"`.

### TINA 4.0.0

- No documentation of exit codes was found in any manual page, README, INSTALL or CHANGES (grep for "exit code", "exit status", "return code", "status code" is empty). Exit status is UNVERIFIED for all tools; the adapter should parse stdout and treat the exit code only as "non-zero means failure" after CALIBRATE confirms it.
- Machine-readable output: textual only for `tina`/`sift` summaries (parse text). `tina` can also write `.ktz`/`.aut`/`.mec`/`.bcg` transition systems. `struct` has JSON output (`-j`, `-j2`, `-j3`). `scan -b` prints TRUE/FALSE and `-script` prefixes the formula (`-id s` replaces the prefix), designed for scripting; `selt -q -b` prints just the truth value (selt.txt, scan.txt).
- Timing: `tina -stats` / `sift -stats` print progress on the fly; `selt` with `-v` (default) "prints banner and evaluation times for LTL commands". No documented total-runtime field for `tina`/`sift`. Use external wall clock and RSS. Limits: `-t SECONDS` (duration of enumeration), `-c N` (markings), `-b N` (place bound), `-en N` for tina; `-c`, `-t`, `-b` for sift. Output on a limit hit is UNVERIFIED. The tina `-d` flag disables the digest; `digestfile` argument (positional) redirects it.

## Open items for the first run on the NixOS box (CALIBRATE)

1. Does LoLA 2.0 accept `ndrio -lola` output unchanged; what happens with `-r -d`.
2. Exact text of tina/sift/scan result lines for Q0 to Q6, QR, QC and what a limit hit looks like for each tool.
3. TINA exit codes.
4. Equivalence of the single-formula variants of Q2, Q3, Q5 with the per-place/per-transition loops in LoLA.
5. Whether `scan` and `selt` accept omega-weighted ktz from `tina -C` (QC on unbounded nets).
6. TINA `-R -s 1` can stop a marking graph on an "unbounded" sufficient test; for Q0 and exhaustive QR on a bounded net use `-s 0` with `-c`/`-t` limits so the run cannot silently stop early.

## Verified on the NixOS box (2026-10-23)

Real runs of LoLA 2.0 and TINA 4.0.0 on two tiny nets (`cyc`, `dead`) and the MCC model Anderson-PT-04; verbatim outputs are in
`evaluation/petrivet-eval/testdata/tools/` and are the test inputs of the parsers (`adapters/lola.rs`, `adapters/tina.rs`).

Verified:
- `ndrio -PNML -lola in.pnml out.lola` produces a file LoLA 2.0 accepts unchanged; place ids are kept as names; exit 0. On invalid XML ndrio prints
  `unexpected error in pnml parser (...)` on stderr, exits 1 and leaves an EMPTY output file (tina and sift behave the same on stdout/stderr). A failed
  conversion is therefore "non-zero exit or empty or missing output file" (`adapters/convert.rs`).
- LoLA: `--check=full --stubborn=off --quiet --json=/dev/stdout` gives `analysis.stats.states` and `.edges` (Q0). `--formula="EF DEADLOCK"` gives
  `analysis.result` true iff a deadlock is reachable (Q1 is its negation). `--formula="AGEF FIREABLE(t0)"` works. The exit code is 0 for every completed
  analysis, yes or no. On an invalid net: exit 1, stderr `lola: syntax error -- aborting [#01]` (preceded by an echo of the offending line, so the
  adapter takes the first line starting with `lola:`) and a JSON object WITHOUT an `analysis` key.
- TINA: `tina -R -q net.pnml` prints a text report: lines `bounded`, `N marking(s), M transition(s)` (states and edges), `live` or `not live`,
  `N dead marking(s), K live marking(s)`, `N dead transition(s), K live transition(s)`. `sift -R -dead -q net.pnml` prints
  `some state violates condition -f:` (plus lines) only if a deadlock exists. `sift -R -rs -k net.pnml net.ktz` followed by
  `scan net.ktz -r "p0 = 0 /\ p1 = 1" -b` prints `TRUE` or `FALSE`. Every TINA tool exits 0 on completed runs and 1 on parse errors.
- Anderson-PT-04: LoLA and TINA both give 29641 states and 97516 edges (matches the oracle), no deadlock, live.

Still UNVERIFIED (implemented as described above, each marked `UNVERIFIED: settled by calibrate on the box` in the code):
- Q2 on unbounded nets: LoLA `IMPOSSIBLE (... = oo)` with `--search=cover` (and whether a null result occurs), and what `tina -R -q` prints for an unbounded
  net (the adapter reads `bounded` / `not bounded`; any other text is UNKNOWN with the raw line as detail).
- Q5 and Q6 on TINA: `sift -R -f safe -q` and its violation line; `sift -R -k` plus `scan -n "p = m" -b` (TRUE iff the place has that value everywhere).
  Place names that are not plain identifiers are written `{name}` in selt formulas (also unverified).
- QC: LoLA `EF (p >= a AND ...)` on unbounded nets (termination), TINA `scan -r "p >= a /\ ..." -b` on the marking graph.
- Behaviour on limit hits (there are no tool-side limits; the external wall limit applies) and exit code 2 of LoLA (mapped to UNKNOWN).
- The combined LoLA formulas of Q2, Q3 and Q5 (parse and agree with the per-place / per-transition loops).
- Q4 and Q6 on LoLA as one `sh -c` script with early exit.

`petrivet-eval calibrate` settles all of these: it runs every solver on the 30 hand-checked nets in `evaluation/calibration/` and prints, per solver, a
table (agree / disagree / unknown / not_supported / input_rejected / crash / timeout / runner_error per query) and a block with command line and output
prefixes for every disagreement and every failure. A disagreement or a failure on a net in scope means the formula or the reading above is wrong and is
fixed before RUN; a query that a tool cannot answer in the sense of the spec becomes `NOT_SUPPORTED` for it.
