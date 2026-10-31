== Behavioral Properties <impl-behavioral-properties>

The following sections discuss the API design and implementation of `petrivet`
for the behavioral properties discussed in @background-behavioral-properties,
following the guiding principles outlined in @solution-design.

`petrivet` attempts to dispatch to the most efficient known procedure to answer the user's query,
while respecting the specified cost-control mechanisms (timeout and/or efficiency restriction).
This is done on a best-effort basis: which procedure will yield the answer fastest is generally not known for sure,
and we must rely on heuristics.
Some procedures are complete and will always return a definitive answer,
while others are incomplete, and we might need to fall back on a more expensive procedure if the first one does not yield a definitive answer.

`petrivet` also caches the results of some property checks to save time on subsequent queries.

Each of the next sections describes the scope and granularity of a behavioral property,
followed by the design of `petrivet`'s API for checking that property,
and then the implementation backing the API, including the algorithms and data structures used.

#include "01-boundedness.typ"
#include "02-liveness.typ"
#include "03-deadlock-freedom.typ"
#include "04-reachability-coverability.typ"
