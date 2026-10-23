# Calibration: hand answers versus petrivet

Hand answers (`answers.json`) were derived first, then compared with
`/Users/michael.dyer/RustroverProjects/petrivet/target/release/petrivet analyze FILE -p <prop> -m general|dispatch --json`
(binary dated 2026-10-22 17:26, one commit older than the library HEAD at 18:52; rebuild before adjudicating).
628 checks (queries Q0-Q6, QR, QC, class detection, both modes). No hand answer was changed after the comparison.

Status update (2026-10-23): items 1 and 2 below were petrivet defects with one common cause and are RESOLVED by the library
commit `efb382d` ("fix dropped liveness verdict and panic"). Items 3 and 4 still stand. The original findings are kept unchanged
below, each with its resolution.

## Direct contradictions (true against false, or different counts)

None. All 30 structural classes detected equal the intended class, Q0 (states and edges) matches for all 25 bounded nets,
and `statespace -m general` reports `unbounded` for the 5 unbounded nets.

## Candidate petrivet bug

1. **RESOLVED by `efb382d`.** `cal-11-mg-unbounded-pipe`, `-m dispatch`, properties `bounded` and `onesafe`: the CLI panics
   (`petrivet/src/api/system/boundedness.rs:282:60: called Option::unwrap() on a None value`, verdict `error`, exit 1).
   Hand answer: bounded = false (place p4 gets a token on every lap of the marked circuit p0,t0,p1,t1), 1-safe = false.
   `-m general` answers `bounded` = false correctly. The net is a marked graph that is not strongly connected
   (t1 feeds p4, p4 feeds t2, the circuit t2,p3,t3,p2 is unmarked). Petrivet is wrong here (a panic is never a verdict); my answer is not in doubt.
   Resolution: the structural boundedness procedure for marked graphs called `self.is_live().call().unwrap()`, and `is_live()` returned `None`
   for this net because of the bug in item 2; the `unwrap()` is now `== Some(true)`. Regression test:
   `unbounded_marked_graph_that_is_not_live_is_answered` in `api/system/boundedness.rs`.

## Incompleteness, not disagreement (petrivet says `unknown` or `not_supported`, never a wrong verdict)

2. **RESOLVED by `efb382d`** (cause found: `Liveness::aggregate` in `core/system/liveness.rs` took the maximum of the transitions' upper
   liveness bounds instead of the minimum, so one dead transition next to live ones left the net's level undecided).
   `-m dispatch`, properties `live` / `quasi-live` return `unknown` on non-live nets that have a definite hand answer
   (and agree with `-m general`): cal-06 (L F, QL F), cal-08 (L F, QL T), cal-15 (L F, QL T), cal-23 (L F, QL T),
   cal-11, cal-21, cal-28 (unbounded). Hand answers are simple (see `why` in answers.json); the dispatch procedure
   evidently only proves positive results for these classes. An adapter must report `unknown` as unknown, not as false.
   Resolution: with the aggregate bound corrected, dispatch answers `false` on cal-06, 08, 15, 23 and on the unbounded marked graph cal-11.
   Check (a scratch copy of the library with the same one-line change, run through the CLI on all 30 nets, dispatch mode, Q1-Q5):
   134 correct, 14 unknown, 2 errors before; 146 correct, 4 unknown, 0 errors after; no wrong verdict in either run. The four remaining
   unknowns are `live` and `quasi-live` on the unbounded, non-structural nets cal-21 (extended free choice) and cal-28 (general), where
   petrivet has no applicable procedure (this is the expected `unknown`, outside the SPEC section 3 scope). The same two MCC models that showed
   the problem in a smoke run (AutonomousCar-PT-02a, Angiogenesis-PT-01) now answer `false` in dispatch mode, as in the MCC oracle.
   Regression tests: `one_dead_transition_makes_the_net_neither_live_nor_quasi_live`, `dead_transition_answers_do_not_depend_on_query_order`
   (`api/system/liveness.rs`) and `net_level_is_the_minimum_over_its_transitions` (`core/system/liveness.rs`).
3. `-m dispatch`, property `stable` and `statespace`: `not_supported` (documented: general mode only).
4. `-m general` on the 5 unbounded nets (cal-11, 19, 20, 21, 28): `unknown` for deadlock-free, live, quasi-live, stable and for
   the QR targets. Hand answers exist (decidable by the explicit reachable family listed per net), but SPEC section 3 puts these
   outside the evaluation scope; QC targets on unbounded nets are answered correctly in both modes.

## Hand-side notes (places where definitions matter, not disagreements)

- Q6 is read literally: a place that is never marked is stable (value 0). cal-06 (p2), cal-02, cal-16 and cal-11 (p2, p3) rely on this.
  If the adapters (LoLA StableMarking, TINA) use a stricter reading, that is an adapter definition question, not a petrivet bug.
- Q0 "transitions" counts pairs (marking, enabled transition). No calibration net has two transitions leading from the same marking to the same marking,
  so counting distinct (marking, successor) pairs gives the same numbers.
