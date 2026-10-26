#import "./01-introduction/03-research-questions.typ": *

= Conclusion <conclusion>

I have presented `petrivet` #footnote[https://github.com/MichaelOwenDyer/petrivet].
This paper aimed to answer the following research questions:

#rqs.existing-tools (#rqs.existing-tools-text) was answered in @related-work.

#rqs.correctness (#rqs.correctness-text) was answered in @eval-correctness.

#rqs.procedure-dispatch (#rqs.procedure-dispatch-text) was answered in @eval-dispatch.

#rqs.cost-control (#rqs.cost-control-text) was answered in @eval-dispatch.

#rqs.perf-mem (#rqs.perf-mem-text) was answered in @eval-perf-mem.

== Future Work

The future work on `petrivet` will include extensions to both its core procedures, unlocking new levels of performance,
and additions to its scope, adding support for extensions on the basic Petri net model.

- IC3 is a model checking procedure that has been applied to Petri nets @hassan_better_2013 @kloos_incremental_2013
which takes a different approach to model checking than explored so far. This would be one more tool in the toolbox
for analyzing Petri nets, and it would be interesting to see how it performs compared to the existing procedures.
- The current implementation of petrivet only supports ordinary Petri nets.
There are many extensions to the basic model which could be supported in the future.
In particular arc weights and place capacities are two common extensions which do not increase the complexity of the
underlying theory (every Petri net with arc weights and / or place capacities can be transformed into an equivalent ordinary Petri net),
but do increase the expressiveness of the model (that transformation may result in a significant increase in the size of the net).
This would allow for a more natural representation of certain systems, but would also require additional work to support these extensions in the core logic.
In particular, the various structural properties of Petri nets do not apply in general to Petri nets with arc weights and
place capacities, so the current optimizations for specific subclasses of nets would not be correct.
- structural reductions @berthelot_reduction_1976 @thierry-mieg_structural_2020

- Model checking is a formal verification technique that systematically explores the state space of a system to verify
  whether certain properties hold. It involves constructing a model of the system, specifying properties in a formal
  language (such as temporal logic), and using algorithms to check if the model satisfies these properties.
  Model checking can be applied to various types of systems, including hardware circuits, software programs, and
  concurrent systems modeled by Petri nets.
  - Partial Order Reduction
  - LTL and CTL formula checking
  - Binary Decision Diagrams / Hierarchical Set Decision Diagrams for state space representation @couvreur_hierarchical_2005.
- More targeted procedures for general liveness and deadlock-freedom which do not require the construction of the entire reachability graph,
perhaps by reducing to a reachability problem (TODO: cite Hack, Esparza) and applying the existing reachability procedures.
- For Free-Choice Nets, checking liveness and boundedness simultaneously is cheaper than checking them separately, and
is in fact polynomial time @esparza_synthesis_1990 @kemper_efficient_1992.
