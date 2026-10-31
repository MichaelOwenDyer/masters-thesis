#import "../lib.typ": *

== Closing Remarks <impl-closing-remarks>

The dispatch procedures implemented in `petrivet` form the analytical backbone of the library,
and are constantly being improved, extended, and re-evaluated.
Several known results and procedures are not yet implemented, but would likely improve its performance.

*Structural reductions.*
Reduction rules shrink a Petri net while preserving the property of interest,
so that every subsequent procedure operates on a smaller net @berthelot_reduction_1976.
@thierry-mieg_structural_2020[Sec. 5] gives reduction rules for deadlock and safety properties,
and for free-choice nets, the reduction rules of @esparza_analysis_1991 are complete for live and bounded systems.
Applied as a preprocessing step, reductions would benefit every property at once,
including the expensive fallback procedures.

*Polynomial results for subclasses.*
For free-choice nets, liveness alone is co-NP-complete @esparza_petri_2026[Thm. 5.3.11],
but simultaneous liveness and boundedness can be decided in polynomial time by the rank theorem
@esparza_petri_2026[Thm. 5.3.16] @best_petri_2024[Lem. 6.17, Thm. 6.18] @esparza_analysis_1991[Cor. 2.8] @kemper_efficient_1992.
Whenever boundedness is already known or also requested, this would replace the Commoner/Hack criterion,
whose high theoretical complexity often translates to poor performance in practice (@eval-dispatch).
Reachability also has exact characterizations for subclasses which are largely unexplored in the current implementation:
in live marked graphs, a marking is reachable iff it satisfies the marking equation @murata_petri_1989[Thm. 19]
@esparza_petri_2026[Thm. 5.2.7],
and in live and bounded free-choice nets, iff the marking equation has a solution $X$
such that the subnet $N_U$ induced by $U = supp(X)$ contains no trap unmarked in $M_0$ @esparza_petri_2026[Thm. 5.3.19],
which is the same condition that underlies the subnet variant of the "Trap Becomes Marked" refinement in the CEGAR engine
(@impl-reachability-coverability).

*Reusing the CEGAR engine.*
The CEGAR engine of @impl-reachability-coverability could answer more than reachability and coverability queries.
A transition $t$ can fire at least once iff the marking $M(p) = 1$ for $p in pre(t)$ is coverable,
so a coverability query per transition would yield #lv(0) and #lv(1) verdicts even for unbounded nets,
where `petrivet` currently has no procedure at all.
Similarly, generalizing the target from a marking to a linear predicate would allow searching for deadlocks directly,
using the target $and.big_(t in T) or.big_(p in pre(t)) M(p) < C^-(p, t)$,
which would provide the safe procedure for deadlock-freedom of unbounded nets that is currently missing.
The engine itself would also benefit from the extensions discussed at the end of @impl-reachability-coverability:
the subnet variant of the trap refinement, and minimal solutions with jump constraints @wimmel_applying_2011.

*State space exploration.*
`petrivet` currently does not cache state spaces.
This particularly disastrous for Petri nets with less restricted structure and large state spaces,
where the coverability graph fallback is activated for the majority of queries and has to be restarted from scratch each time.
Furthermore, the set of reachable markings cannot be constructed without also constructing the graph structure,
which is unnecessary for queries such as k-boundedness and deadlock-freedom.
Beyond this, the exploration itself could be reduced:
stubborn sets preserve all deadlocks of the full state space @valmari_stubborn_1991,
and are already computed by the guided exploration of the CEGAR engine,
but not used by the general state space explorer.
For coverability, minimal coverability set algorithms @finkel_minimal_1993 @reynier_minimal_2011 @valmari_old_2012
avoid much of the redundancy and $omega$-acceleration cost of the Karp-Miller graph,
and backward search algorithms @abdulla_sat-solving_2004 or IC3 @kloos_incremental_2013 @kang_abstraction-based_2021
offer complete alternatives to it.
For known bounded systems, the fallback procedure could construct the reachability graph instead of the coverability graph,
which would avoid the overhead of $omega$-acceleration where there is no risk of non-termination.
Finally, symbolic representations of the state space, such as decision diagrams @couvreur_data_2002
@couvreur_hierarchical_2005 or partial-order representations such as unfoldings @esparza_unfolding_2001
are established alternatives to explicit exploration for large bounded systems.

