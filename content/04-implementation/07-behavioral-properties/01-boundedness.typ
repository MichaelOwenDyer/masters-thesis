#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *

=== (K-)Boundedness <impl-boundedness>

This section describes the implementation of the (k-)boundedness property of Petri nets within `petrivet`.
See @background-boundedness for a discussion of the property itself.

==== Scope and Granularity

*Scope.* (K-)Boundedness can be checked for the entire Petri net, or for a specific place.

*Granularity.* Boundedness (with or without `k`) can be framed as a yes/no question ("is the Petri net (k-)bounded?"),
but also as a search for the minimal `k` (the exact bound).

==== API

`petrivet` offers two entry points for checking (k-)boundedness of a `PetriNet`, covering all combinations:

+ `boundedness() -> Option<Boundedness>` returns either `Bounded(k)` with the minimal $k$, or `Unbounded`.
  The user can optionally specify the place to check, otherwise the entire Petri net is checked.
+ `is_bounded() -> Option<bool>` corresponds to the yes/no framing.
  In addition to optionally specifying the place,
  the user can also optionally specify a specific value of `k`,
  to check for k-boundedness instead of general boundedness.

Furthermore, both methods also allow specifying a restriction to only use efficient procedures,
in which case they return `None` if no efficient procedure is known for the given net,
and an optional timeout to limit the maximum duration of the analysis.

==== Implementation

`petrivet` descends an array of boundedness procedures ordered by increasing computational cost,
checking each in turn until the query is answered or a cost-control mechanism is triggered.
These procedures sometimes yield exact bounds, and sometimes only upper bounds
(for example, a place is guaranteed 5-bounded, but might also be 4-bounded).
The initial marking also gives a lower bound for the exact bound of a place
(the exact bound of a place which starts with 3 tokens is at least 3).
`petrivet` therefore tracks a lower and upper bound for each place,
and can conclude the exact bound when the lower and upper bounds meet.

*State Machines.*
State machines (including circuits) are conservative, meaning their token count is constant across all reachable markings.
Therefore, they are always bounded, and k-bounded for any k greater than or equal to the initial token count.

Determining the exact bound of a place requires a case distinction, however: if the net is strongly connected,
then all tokens can reach any place, and so every place is exactly k-bounded for k equal to the initial token count.

If the net is not strongly connected, then not all tokens can reach every place.
We can determine the exact bound of a place $p$ by computing $M_0(prestar(p))$:
the number of tokens in the initial marking that can reach $p$ @murata_petri_1989.

*Marked Graphs.*
Live marked graphs, as discussed in @background-structural-classes,
are bounded if every place lies on a circuit.
Every place is bounded by the minimum number of tokens on the circuits it lies on,
and unbounded if it does not lie on any circuit @esparza_petri_2026[Thm. 5.2.4].
If the marked graph is not live, then some circuits are unmarked,
which can prevent tokens on other circuits from circulating,
leading to some places being bounded by a lower number than the minimum of the circuits they lie on.
`petrivet` therefore computes the minimum number of tokens on the circuits a place lies on, and records this as exact
if the marked graph is live, and as an upper bound if it is not live.
See @impl-liveness-implementation for the procedure used to determine liveness of marked graphs.

*Structural Boundedness.*
If the query has not been answered by the above procedures,
then `petrivet` searches for a P-subinvariant which covers the places of interest $S subset.eq P$,
using the encoding $phi_"kind" and phi_"cover"(S)$ from @impl-p-invariants with $bowtie = {<=}$.
For queries about the entire Petri net, $S = P$.
This is a linear programming problem, and can be solved in polynomial time by an SMT solver.
If a solution $y$ exists, then the weighted token sum $y dot M$ can never exceed its initial value $y dot M_0$,
and since every place $p in S$ has weight $y_p >= 1$, it is bounded by $⌊(y dot M_0) / y_p⌋$ for any $M_0$
(_structural_ boundedness) @best_petri_2024 @murata_petri_1989.
This is only a semi-decision procedure: the lack of a P-subinvariant gives us no information.

*Exploring the Coverability Graph.*
As a last resort, `petrivet` can explore the coverability graph of the Petri net, which is guaranteed to provide full
information about the boundedness of every place, but is expensive to compute.
The stopping condition for the exploration depends on the query:
`is_bounded()` queries can stop with the verdict `false` as soon as a (or the) place is found to be unbounded
(or not k-bounded); otherwise the whole graph must be explored.
`boundedness()` queries must explore the entire coverability graph to find the exact bounds.