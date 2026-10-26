#import "../lib.typ": *
#import "../eval-data.typ": *

== Method <eval-method>

This section describes the methodology by which the evaluation was conducted.

=== Queries

All measured experiments used the same nine Petri net queries.
They all take a Petri net $(N, M_0)$ as input, and return a boolean answer.
Two (reachability and coverability) additionally take a target marking as input.

The queries are listed here:
- Deadlock-free: Is $(N, M_0)$ deadlock-free?
- Bounded: Is $(N, M_0)$ bounded?
- Safe: Is $(N, M_0)$ 1-bounded?
- Live: Is $(N, M_0)$ #lv(4)-live?
- Quasi-live: Is $(N, M_0)$ #lv(1)-live?
- Stable place: Does $(N, M_0)$ contain a place whose marking is constant in all reachable markings?
- Reachable: Is $M_t$ reachable in $(N, M_0)$?
- Coverable: Is $M_t$ coverable in $(N, M_0)$?
- State space size: How many nodes and edges (markings and transition firings) are in the reachability graph of $(N, M_0)$?

=== Net collections

Four sources of Petri nets were used in the evaluation.
Results are reported separately for each collection because they have different characteristics and are used for different purposes.
The following table summarizes the four collections, their sizes, the source of the reference answers, and the purposes for which they are used.

#figure(
  table(
    columns: (auto, auto, 1fr, 1fr),
    align: (left, left, left, left),
    table.header[*Collection*][*Size*][*Reference answers from*][*Used for*],
    [Calibration nets], [#val("e0.calibration.n_manifest_lines")], [Worked out by hand], [Validating the evaluation setup],
    [Randomized nets], [#fmt(val("e0.gen.n_manifest_lines"))], [Agreement of two reference tools], [Correctness],
    [Parametrized nets], [#val("e0.fam.n_manifest_lines") nets \ across 15 families], [By construction], [Dispatching, performance and memory comparison, cost control],
    [2026 MCC #footnote[https://mcc.lip6.fr/] <mcc2026> models], [#val("e0.mcc_small.n_manifest_lines") small, #val("e0.mcc_large.n_manifest_lines") large], [Published verdicts #footnote[https://yanntm.github.io/pnmcc-models-2026]], [Correctness, performance and memory comparison],
  ),
  caption: [The four collections of nets used in the evaluation.],
)

*Calibration nets.* Thirty small Petri nets were constructed and analyzed by hand.
They serve two purposes: to validate the measurement setup for the various tools, and to provide answers which do not depend on any software.

*Randomized nets.* A pseudorandom number generator was used to generate 100 nets each across six structural subclasses
(circuits, state machines, marked graphs, extended free-choice nets, asymmetric choice nets, and general nets) (@background-structural-classes),
and three size ranges (2-5 places, 6-8 places, 9-12 places). This produced 100 x 6 x 3 = 1800 nets.
Duplicates were identified by their content hash and pruned,
and a few combinations of structural class and size range yielded fewer than 100 nets,
leaving the final count at #val("e0.gen.n_manifest_lines").
The generators for each structural class occasionally produced nets which also belonged to a more specific class,
and in these cases the net was classified into the most specific class.
The RNG was seeded, so the collection can be reproduced exactly, and the files are archived together with a checksum.

*Parametrized nets.* Fifteen families of nets parametrized by size.
The answers to every query are known for every family by construction.
These families are used to evaluate how running time grows with the size of the net.
@tab-parametrized-families lists the families, the properties they satisfy, and a small example of each.

*MCC models.* From the model repository of the 2026 Model Checking Contest #footnote[https://mcc.lip6.fr/models.php]
I drew a seeded random sample of 90 "small" and 20 "large" non-colored Petri nets, where
a small net is one whose state space is known to be at most $10^6$ markings, and a large net is one whose state space is larger than that or unknown.
Models which `petrivet` failed to parse were skipped, which included:
#val("e0.mcc.small.rejected_not_connected") small and #val("e0.mcc.large.rejected_not_connected") large non-connected nets,
#val("e0.mcc.small.rejected_arc_weight") small and #val("e0.mcc.large.rejected_arc_weight") large nets which contained nonstandard ($!= 1$) arc weights, and
#val("e0.mcc.large.rejected_too_large") large nets whose file size exceeded 5 MB.

#figure(
  table(
    columns: (auto, auto, auto, auto, auto, auto, 30%),
    align: (left, left, left, left),
    table.header[*Family*][*Description*][*DF*][*B*][*L*][*QL*][Example],
    [circuit],
    [A simple ring of $n$ places and $n$ transitions with one token.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/circuit.svg")],

    [sm-hub-and-spokes],
    [A state machine with one central place and $n$ spokes, with $n$ tokens.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/sm_hub_and_spokes.svg")],

    [sm-two-cycles],
    [Two disjoint rings of $n/2$ places each, joined by a one-way transition, with one token.],
    [yes], [yes], [no], [yes],
    [#image("/figures/nets/families/sm_two_cycles.svg")],

    [sm-two-cycles-n-tokens],
    [Like sm-two-cycles, but with $n$ tokens in the first cycle.],
    [yes], [yes], [no], [yes],
    [#image("/figures/nets/families/sm_two_cycles_n_tokens.svg")],

    [sm-choice-ring],
    [A ring of $n$ sequential two-way decisions with a single token.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/sm_choice_ring.svg")],

    [sm-ring-with-sink],
    [A ring of size $n$ with $n/2$ tokens, plus a dead-end.],
    [no], [yes], [no], [yes],
    [#image("/figures/nets/families/sm_ring_with_sink.svg")],

    [mg-buffer-ring],
    [An outer counter-clockwise ring with one token and an inner clockwise ring using the same $n$ transitions.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/mg_buffer_ring.svg")],

    [mg-buffer-ring-unmarked-circuit],
    [Like mg-buffer-ring, but containing an unmarked circuit.],
    [no], [yes], [no], [no],
    [#image("/figures/nets/families/mg_buffer_ring_unmarked_circuit.svg")],

    [mg-forkjoin-cycle],
    [$n$ concurrent branches of two places each.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/mg_forkjoin_cycle.svg")],

    [mg-forkjoin-unbounded],
    [A source transition puts token onto $n$ places and a sink transition consumes them.],
    [yes], [no], [yes], [yes],
    [#image("/figures/nets/families/mg_forkjoin_unbounded.svg")],

    [efc-forkjoin],
    [A ring of $n$ sequential two-way parallel decisions with $n/2$ tokens.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/efc_forkjoin.svg")],

    [efc-forkjoin-leaky],
    [Like efc-forkjoin, except that one of the choices does not produce enough tokens to continue.],
    [no], [yes], [no], [yes],
    [#image("/figures/nets/families/efc_forkjoin_leaky.svg")],

    [ac-cycle],
    [$n$ conjoined segments with a left place and a right place each. The left tokens can move independently, the right tokens can only move together with the left tokens. Contains one left and one right token.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/ac_cycle.svg")],

    [ac-cycle-no-low-token],
    [Like ac-cycle, but without the right token.],
    [yes], [yes], [no], [no],
    [#image("/figures/nets/families/ac_cycle_no_low_token.svg")],

    [general-cycle],
    [A ring of $n$ segments with two places each (an a-place and a b-place). Per segment there are two transitions that move one token forward independently and a shared transition that moves both together. There is one token on each of the first a and b places. The shared transition overlaps with both independent ones in a way no restricted class allows, so the net is general.],
    [yes], [yes], [yes], [yes],
    [#image("/figures/nets/families/general_cycle.svg")],
  ),
  caption: [
    The fifteen families of parametrized nets used in the evaluation, with their properties and a small example.
    DF = deadlock-free, B = bounded, L = #lv(4)-live, QL = #lv(1)-live (quasi-live).
  ],
) <tab-parametrized-families>

=== Reference answers <eval-reference>

For the authoritative reference answers, I used a different strategy for each of the four collections.
The calibration nets were analyzed by hand (and cross-checked by all tools during the evaluation),
the randomized nets were analyzed by two reference tools of unrelated origin (LoLA #footnote[https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/] and TINA #footnote[https://projects.laas.fr/tina/index.php]) for consensus,
the parametrized nets were constructed with the answers known by design,
and the MCC model verdicts were taken from the official oracle #footnote[https://yanntm.github.io/pnmcc-models-2026].

For the randomized nets, a disagreement or lack of answer from either reference tool within the 60s time limit was
treated as a failure to produce an authoritative answer, and the net was excluded from the correctness evaluation.
These exclusions happened before any `petrivet` runs, so that `petrivet`'s answers could not influence which Petri nets
entered the correctness evaluation.

There is one exception to this rule: for the reachability and coverability queries, TINA does not finish within a
reasonable time. In these cases, a definite answer from LoLA alone is used as the reference, and these cases are labeled
separately in the results. Any disagreement between `petrivet` and a reference answer produced by LoLA alone is first
checked by hand.

=== Tools under comparison <eval-tools>

`petrivet` enters the evaluation in three different configurations through a CLI wrapper:
+ In the default mode ("automatic choice"), `petrivet` chooses the procedure to use based on its internal dispatching logic.
+ In the efficient-only mode ("fast methods only"), `petrivet` must answer the query using only polynomial-time procedures (if possible).
+ In the general mode ("plain exploration"), `petrivet` must answer the question by brute-force exploration of the
  reachability graph, which isolates the effect of the internal dispatching. As discussed in @background-reachability, this is an infinitely large structure for unbounded Petri nets and in many cases
  results in a timeout in the evaluation.

LoLA 2.0 and TINA 4.0.0 are used as external reference tools, packaged reproducibly as Nix derivations.
A third external tool, ITS-Tools, the overall winner of the 2026 Model Checking Contest, is included as an extra point
of comparison for performance and memory, but is not used as a correctness reference due to difficulties supporting
all of the queries. It answers five of the nine queries which overlap with the MCC's examinations (state space size,
deadlock-freedom, quasi-liveness, 1-safety, and stable place); the others are either not supported or cannot be answered
in time: for liveness it spends a fixed 30 seconds on an internal procedure (which invariably consumes the entire evaluation
time limit), and for the state space size query it only reports the number of markings and not the number of transition
firings. This second limitation is a fundamental design trade-off: while `petrivet`, `LoLA` and `TINA` are all explicit
state model checkers which fire transitions and count the resulting markings,
ITS-Tools is a symbolic model checker which constructs the state space indirectly by manipulating hierarchical set decision
diagrams (SDDs) @couvreur_hierarchical_2005. This gives it a flatter performance curve particularly for large
nets, but its JVM-based implementation gives it a higher constant overhead.

Each external tool was wrapped with an adapter which translates the evaluation query into the tool's own input format,
and translates the tool's output back. Each adapter was validated against the calibration nets to ensure that the translation was
correct and that the tool was invoked correctly.
Where a tool offers no direct query (for example, LoLA has no direct API for the liveness of a whole net),
the translation combines several smaller queries; these are described in the appendix. TODO reference

All tools run as separate processes, under the same time and memory limits, on the same machine.
Using a command-line interface for all tools places them on equal footing, including process start-up overhead.
The start-up time of each tool is measured on a trivial net and reported next to the results,
so that it can be subtracted where the analysis itself is short.

=== Measurement setup <eval-setup>

Measurements were conducted on a single desktop machine with a four-core, eight-thread Intel i7-4790K CPU @ 4.00GHz and 32 GiB of RAM.
At most three runs were executed at a time, each pinned to a unique physical CPU core with the sibling hyperthread left idle.
Turbo frequency boosting was switched off and the frequency governor fixed at "performance" to avoid variability in the
clock speed with load.
Graphical sessions, file synchronization and other background services were stopped for the measured runs.

Each run has a wall-clock time limit of 30 seconds, enforced outside the process, and a memory limit of 8 GiB.
A run that exceeds either limit is recorded as a time-out or memory-out, and the process is killed.
A run that ends abnormally, through a crash, abort, or other error, is recorded under its own category.
Timing runs are repeated; the number of repetitions was fixed in a short pilot study before the measured runs began.
Settings, tool versions, and processor configuration are recorded with every run.

Correctness runs are not timed; tools are given 60 seconds to provide an answer which is then compared to the reference answer.

Peak memory is measured in a second pass over the runs which produced an answer, using a separate low-overhead wrapper
that records the peak resident set size of the process.
A separate low-overhead wrapper is needed to lower the measurement floor as far as possible, because the wrapper's own
memory use is included in the peak measurement of the tool process.
This measurement floor is about #dec(val("e9.lola.baseline_rss_kib") / 1024, d: 1) MiB.
For a tool which spawns child processes of its own, such as ITS-Tools, the measured value is that of the largest single
process and not the sum.
A run which timed out does not report a peak memory value.
A run which ran out of memory keeps the value of the limit, 8 GiB, as its peak memory.

=== Analysis <eval-analysis>

Every table and figure below is generated from the raw measurement data by a Python script.
Timings are summarized by medians and interquartile ranges.
For head-to-head comparisons of running times I use the paired Wilcoxon signed-rank test, which makes no assumption
about the distribution of times, with Holm's correction across the structural classes,
and I report the Hodges-Lehmann estimate of the shift together with the median ratio. TODO Cite
Timeouts are treated as censored data: their number is reported, and the comparison is repeated without them to see if
the conclusion is robust to their presence.
For the tool comparisons, I report the number of solved instances, the penalized average runtime where a failed run
counts as twice the time limit, and cactus plots.
Where no disagreement with the reference answer is found, I provide the one-sided 95 percent upper bound on the true
rate of disagreement (Clopper-Pearson interval).
All tests are listed together with their correction method // TODO: what does this mean?
and unsupported hypotheses are explicitly marked as such.