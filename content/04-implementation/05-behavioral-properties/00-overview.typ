== Behavioral Properties (Overview) <impl-behavioral-properties>

The following sections discuss the API design and implementation of `petrivet`
for the behavioral properties discussed in @background-behavioral-properties,
following the guiding principles outlined in @solution-design.

`petrivet` attempts to dispatch to the most efficient known procedure to answer the user's query,
while respecting the specified cost control mechanisms (timeout and/or efficiency restriction).
This is done on a best-effort basis: which procedure will yield the answer fastest is generally not known for sure,
and we must rely on heuristics.
Some procedures are complete, meaning they will always return a definitive answer, while others are incomplete,
meaning they may not provide an answer and we must fall back to a more expensive procedure to try to obtain an answer.

`petrivet` also caches the results of some property checks to save time on subsequent queries.

For each property, the scope and granularity dimensions are listed,
yielding the total number of variants of the check that the API provides.

#include "01-boundedness.typ"
#include "02-liveness.typ"
#include "03-deadlock-freedom.typ"
#include "04-reachability-coverability.typ"
