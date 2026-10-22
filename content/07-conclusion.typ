= Conclusion <conclusion>

I have presented _petrivet_ #footnote[https://github.com/MichaelOwenDyer/petrivet].
This paper aimed to answer the following research questions:

Research Question 1 (To what extent do existing Petri net analysis tools meet the requirements of tool builders
(programmatic access, safe portability, cost control, model interchange, adequate performance, and traceability to theory),
and where do they fall short?) was answered in @related-work.

RQ2 "How can a library organize Petri net theorems so that each query is dispatched to the cheapest applicable procedure,
the caller can restrict its runtime cost, and each answer is traceable to the theory that made it possible?"
was answered in @solution-design and @implementation.

RQ3 "To what extent does the resulting artifact implement the applicable theorems from the literature soundly and efficiently,
and how well does it meet the requirements of tool builders?"
was answered in @evaluation.

RQ4: Which design principles emerged during implementation, and what trade-offs did they involve?
was answered in @discussion.

== Future Work

The future work on _petrivet_ will include extensions to both its core procedures, unlocking new levels of performance,
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
