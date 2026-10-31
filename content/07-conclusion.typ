#import "./01-introduction/03-research-questions.typ": *

#pagebreak(weak: true)
= Conclusion <conclusion>

This thesis presented `petrivet` #footnote(<petrivet>),
a Rust library for analyzing place/transition Petri nets
designed to meet the requirements of a process-orchestration setting.
It is called from code,
supports model interchange via PNML,
allows bounding the cost of every query,
returns evidence with its answers,
and returns "unknown" where it has no answer.
It has procedures for k-boundedness, liveness, deadlock-freedom, reachability, and coverability,
dispatching each query to the most efficient known procedure that can answer it.

The research questions were answered as follows.

- *#rqs.existing-tools*: "#rqs.existing-tools-text" \
  No surveyed tool meets all eight requirements (@related-work).
- *#rqs.correctness*: "#rqs.correctness-text" \
  No verdict of `petrivet` contradicts a reference verdict across all evaluated queries (@eval-correctness),
  after fixing the three bugs found during the evaluation.
- *#rqs.procedure-dispatch*: "#rqs.procedure-dispatch-text" \
  Dispatch makes the more restricted net classes easier to analyze, often by orders of magnitude,
  but costs time on the more general nets (@eval-dispatch).
- *#rqs.cost-control*: "#rqs.cost-control-text" \
  The restriction to polynomial procedures can return verdicts where brute-force times out,
  and deadlines are kept within 200 ms for the typical query,
  though some stages which do not check the deadline internally can cause overshoot (@eval-cost-control).
- *#rqs.comparison*: "#rqs.comparison-text" \
  `petrivet` is competitive with LoLA, TINA, and ITS-Tools for liveness and on the parametrized families,
  but behind them on deadlock-freedom and quasi-liveness on MCC models (@eval-comparison).

The interpretation of these results, and lessons learned during implementation, are discussed in @discussion.

== Future Work

The most direct improvements to `petrivet` concern performance on nets of less restricted structure.
Known procedures that are not yet implemented, among them structural reductions,
the rank theorem for free-choice nets, partial-order reductions, and minimal coverability set algorithms,
are listed in @impl-closing-remarks.
Property-directed reachability (IC3) @hassan_better_2013 @kloos_incremental_2013
would add a complementary approach to the existing CEGAR engine.

The scope of `petrivet` can be extended beyond just ordinary Petri nets.
Arc weights and place capacities are common extensions with the same analytical expressiveness as ordinary nets,
but enhanced succinctness, and would allow for a more natural representation of certain systems.
Colored Petri nets @van_der_aalst_strategies_2013 are particularly useful in the context
of business process orchestration, and would allow for a more direct representation of the systems
that motivated this work.
Temporal-logic model checking (LTL and CTL) would cover a wider range of queries than the
current set of behavioral properties can express.
Symbolic representations of the state space, such as decision diagrams @couvreur_hierarchical_2005,
would allow for more efficient exploration of large bounded systems.

Finally, `petrivet` should be validated again outside this thesis
by participating in upcoming editions of the Model Checking Contest @kordon_results_2026,
by language bindings that allow it to be used from other programming languages,
and by adoption in real-world applications.