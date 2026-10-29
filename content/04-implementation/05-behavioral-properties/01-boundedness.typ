#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *

== (K-)Boundedness

This section describes the implementation of the (k-)boundedness property of Petri nets within `petrivet`.
See @background-boundedness for a discussion of the property itself.

(K-)Boundedness varies along two dimensions:

*Scope*. (K-)Boundedness can be checked for the entire Petri net, or for a specific place.

*Granularity*. Boundedness (with or without `k`) can be framed as a yes/no question ("is the Petri net (k-)bounded?"),
but also as a search for the minimal `k` (the exact bound).

`petrivet` offers two entry points for checking (k-)boundedness of a `PetriNet`, covering all combinations:

+ `PetriNet::boundedness() -> Option<Boundedness>` returns either `Bounded(k)` with the minimal $k$, or `Unbounded`.
  The user can optionally specify the place to check, otherwise the entire Petri net is checked.
+ `PetriNet::is_bounded() -> Option<bool>` corresponds to the yes/no framing.
  In addition to optionally specifying the place to check, the user can also provide a specific value of `k`,
  to check for k-boundedness instead of general boundedness.

Furthermore, both methods also allow specifying a restriction to only use efficient procedures,
in which case they return `None` if no efficient procedure is known for the given net,
and an optional timeout to limit the maximum duration of the analysis.

=== Procedure Dispatch

`petrivet` descends an array of boundedness procedures ordered by increasing computational cost,
checking each in turn until the query is answered or a cost-control mechanism is triggered.
These procedures sometimes yield exact bounds, and sometimes only upper bounds
(for example, a place is guaranteed 5-bounded, but might also be 4-bounded).
The initial marking also gives a lower bound for the exact bound of a place
(a place which starts with 3 tokens is guaranteed to be at least 3-bounded).
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
Marked graphs, as discussed in @background-structural-classes, are bounded if and only if every place lies on a circuit.
Every place is bounded by the minimum number of tokens on the circuits it lies on,
and unbounded if it does not lie on any circuit.
However, this bound is only exact if the marked graph is live @esparza_petri_2026.
If it is not live, then some circuits are unmarked, which can prevent tokens on other circuits from circulating,
leading to some places being bounded by a lower number than the minimum of the circuits they lie on.
`petrivet` therefore computes the minimum number of tokens on the circuits a place lies on, and records this as exact
if the marked graph is live, and as an upper bound if it is not live.
See @impl-liveness-procedure-dispatch for the procedure used to determine liveness of marked graphs.

*Structural Boundedness.*
If the query has not been answered by the above procedures,
then `petrivet` checks for the existence of a P-subinvariant which covers the places of interest.
This is formulated as a linear programming problem, and can be solved in polynomial time.
If such a P-subinvariant $y in ℕ^{|P|}$ exists, then the places in its support are (upper) bounded by $⌊(y · M_0) / y(p)⌋$
@best_petri_2024 @murata_petri_1989.
However, it is only a semi-decision procedure: the lack of a P-subinvariant gives us no information.

*Exploring the Coverability Graph.*
As a last resort, `petrivet` can explore the coverability graph of the Petri net, which is guaranteed to provide full
information about the boundedness of every place, but is expensive to compute.
The stopping condition for the exploration depends on the query:
`is_bounded()` queries can stop as soon as a (or the) place is found to be unbounded (or not k-bounded)
with the verdict `false`, otherwise the whole graph must be explored.
`boundedness()` queries must explore the entire coverability graph to find the exact bounds.