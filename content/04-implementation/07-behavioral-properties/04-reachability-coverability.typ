#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *
#import "@preview/lovelace:0.3.0": *

=== Reachability and Coverability <impl-reachability-coverability>

This section describes the implementation of reachability and coverability properties of Petri nets within `petrivet`.
See @background-reachability and @background-coverability for a discussion of the properties themselves.

==== Scope and Granularity

*Scope.* Reachability and coverability are properties of the entire Petri net with respect to a specific target marking.

*Granularity.*
Reachability and coverability can be framed as a yes/no question ("is the target marking reachable/coverable?"),
or, more granularly, as a search either for a witness firing sequence fulfilling the property
or for evidence that no such firing sequence can possibly exist.

==== API

`petrivet` offers two entry points for checking reachability and coverability of a `PetriNet`, covering both combinations:

+ `is_reachable(target: Marking<u32>) -> Option<bool>`
  returns Some(true) if the marking is reachable,
  Some(false) if it is not reachable,
  and None if the analysis was inconclusive.
+ `is_coverable(target: Marking<u32>) -> Option<bool>`
  returns Some(true) if the marking is coverable,
  Some(false) if it is not coverable,
  and None if the analysis was inconclusive.
+ `reachability(target: Marking<u32>) -> Option<Reachability>`
  returns Some(Reachability::Reachable { firing_sequence }) with a witness firing sequence if the target marking is reachable,
  Some(Reachability::Unreachable { contradiction }) with a set of contradictory lemmas if it is not reachable,
  and None if the analysis was inconclusive.
+ `coverability(target: Marking<u32>) -> Option<Coverability>`
  returns Some(Coverability::Coverable { marking, firing_sequence }) with a witness covering marking and firing sequence if the target marking is coverable,
  Some(Coverability::Uncoverable { contradiction }) with a set of contradictory lemmas if it is not coverable,
  and None if the analysis was inconclusive.

`is_reachable()` and `is_coverable()` support both cost-control mechanisms,
while `reachability()` and `coverability()` only support a timeout.
This is because currently no efficient procedures are implemented which can return a witness firing sequence or evidence
of unreachability/uncoverability, and so the efficiency restriction would always return `None` for these methods.
This could be added in the future.

==== Implementation

This section describes the implementation of reachability and coverability together,
as their implementation shares a common engine in `petrivet`.

*Strongly connected state machines.*
Since tokens circulate freely in strongly connected state machines,
a target marking $M$ is reachable from $M_0$ in a circuit iff $M(P) = M_0(P)$,
and coverable from $M_0$ iff $M(P) <= M_0(P)$.
This does not yield a witness firing sequence or evidence of unreachability/uncoverability,
so is only used in the `is_reachable()` and `is_coverable()` methods.

*Maximal unmarked trap.*
If the maximal unmarked trap in the target marking is marked in the initial marking,
then the target marking is unreachable.
This is a direct result of the fact that traps remain marked once they are marked,
and can be checked using the procedure in @impl-traps-siphons.
Note that this only applies to reachability, and not to coverability:
the target marking may not mark an initially marked trap, but the covering marking can mark it.
This also does not yield a witness firing sequence or evidence of unreachability/uncoverability,
so is only used in the `is_reachable()` method.

*Fallback procedures.*
In early versions of `petrivet`, the only fallback approach was simply to explore the reachability graph breadth-first
and check each marking for equality or coverability with the target marking.
While this produces a witness firing sequence for positive results,
and its evidence for the unreachability/uncoverability of the target marking in negative results is absolute,
it is intractable for all but the smallest bounded Petri nets.

The current implementation is more sophisticated.
It is based on the paradigm of counterexample-guided abstraction refinement (CEGAR) @clarke_counterexample-guided_2000,
a well-known technique in formal verification and model checking that has been applied to Petri nets before
@wimmel_applying_2011 @hajdu_new_2015 @thierry-mieg_structural_2020.
CEGAR provides an effective semi-decision procedure for various properties while largely avoiding the state space explosion problem.

The paradigm constructs and iteratively refines a behavioral abstraction (overapproximation) of the original system,
maintaining the invariant that if the property of interest does not hold in the abstraction,
then by construction it cannot not hold in the original system either.
If the property of interest does hold in the abstraction,
then it yields a potential solution which can be checked by simulating it in the original system.
The simulation either confirms the solution or shows it to be spurious,
yielding a counterexample which is used to refine the abstraction and
prevent similarly spurious solutions from being generated in the future.
Repeating this process eventually yields either a solution valid in the original system,
or an abstracted proof that no solution exists:
some combination of insights from the original system eliminates all potential solutions from the abstraction.
@fig-cegar-diagram illustrates the CEGAR paradigm as a BPMN diagram.

#figure(
  image("../../../figures/cegar/diagram.svg"),
  caption: [Counterexample-guided abstraction refinement (CEGAR) paradigm.],
) <fig-cegar-diagram>

CEGAR can be implemented elegantly using a satisfiability modulo theories (SMT) solver such as Z3 @de_moura_z3_2008.
The abstraction is encoded as an SMT problem definition, and checking the property becomes a satisfiability
check using the SMT solver.
Refinements are performed by adding additional constraints, and sometimes additional variables, to the SMT problem definition.
Simulation, the source of counterexamples and the refinements derived from them,
is specific to the type of system being analyzed.

In the case of Petri nets, simulation is not just a matter of firing transitions.
While a witness firing sequence or lack thereof is the ultimate evidence for or counterexample to the property of interest,
there are several (much cheaper) methods to find counterexamples which do not require simulating the Petri net,
and instead only require analyzing the structure of the net with respect to the potential solutions produced by the SMT solver.
These refinements delay confrontation with the state space explosion problem as long as possible.
By arranging these refinement opportunities in order of increasing computational cost,
we can often find cheap wins and avoid expensive simulation entirely,
and if we do have to simulate, the likelihood of the simulation being successful is much higher,
since the potential solution has been refined to be more likely to be valid.

=== CEGAR Engine

`petrivet` implements a CEGAR engine for the reachability and coverability properties of Petri nets,
which backs the API methods described above.
The abstraction is a set of constraints $Phi$, which an SMT solver checks for satisfiability.
Initially, $Phi$ contains one non-negative integer variable $M(p)$ per place $p in P$,
constrained by the target marking $M_"target"$:
$ Phi_0 := and.big_(p in P) M(p) bowtie M_"target"(p) quad "where" bowtie = cases(
  = & "for reachability",
  >= & "for coverability",
) $
The only behavioral difference between reachability and coverability is in the choice of $bowtie$;
all other aspects of the CEGAR engine are identical for both property types.
Every refinement adds constraints which hold in every reachable marking (or every realizable firing sequence),
so $Phi$ over-approximates the set of reachable markings satisfying the target condition,
and if $Phi$ becomes unsatisfiable, then the target marking is unreachable/uncoverable.

Most refinements are implemented as a pair of functions:
one to search for a reason why the solver's current solution is spurious,
and one to encode that insight into $Phi$,
eliminating the spurious solution (and hopefully many others) from the search space.
Others are applied once, preemptively, without any spurious solution.

The engine runs in two phases, shown in @algorithm-cegar.
In the _structural_ phase, $Phi$ only contains the place variables,
so a candidate solution is just a marking $M$, and only structural
refinements (traps and siphons, P-invariants) are applied.
These are cheap, and often enough to prove unreachability.
When no structural refinement can exclude the candidate marking,
the engine enters the _behavioral_ phase:
it adds one integer variable $X(t) >= 0$ per transition together with the marking equation,
so a solution becomes a pair $(M, X)$ of a marking and a Parikh vector.
Besides the structural refinements, this phase applies refinements that reason about $X$,
and finally tries to realize $X$ as a concrete firing sequence in the original Petri net.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [The two-phase CEGAR procedure for reachability and coverability.],
  pseudocode-list(numbered-title: [`CegarDecide`$(N, M_0, M_"target", bowtie)$])[
    + $Phi <- and.big_(p in P) M(p) bowtie M_"target"(p)$
    + $D <- "MaximalSiphonIn"(N, P without supp(M_0))$; #h(0.5em) $Phi <- Phi and and.big_(p in D) M(p) = 0$
    + *loop*: #h(1em) _(structural phase)_
      + *if* $Phi$ is unsatisfiable: *return* unreachable
      + *if* the solver gives up: *return* $bot$
      + $M <- $ the solver's model of the place variables
      + *if* $"MarkedTrap"(M)$ or $"Invariant"(M)$ yields a constraint $psi$:
        + $Phi <- Phi and psi$; *continue*
      + *break*
    + $Phi <- Phi and and.big_(t in T) X(t) >= 0 and "MarkingEquation" and "TransitionOrdering"$
    + *loop*: #h(1em) _(behavioral phase)_
      + *if* $Phi$ is unsatisfiable:
        + *if* the unsat core contains an increment constraint: *return* $bot$
        + *return* unreachable
      + *if* the solver gives up: *return* $bot$
      + $(M, X) <- $ the solver's model of the place and transition variables
      + *if* $"MarkedTrap"(M)$, $"TrapBecomesMarked"(M, X)$ or $"Invariant"(M)$ yields a constraint $psi$:
        + $Phi <- Phi and psi$; *continue*
      + *if* $"Realize"(X)$ finds a firing sequence $sigma$: *return* reachable via $sigma$
      + $Phi <- Phi and "Increments"(X)$
  ]
) <algorithm-cegar>

The refinements are tried in the order listed,
and only the first refinement which excludes the current candidate solution is applied
before the next iteration of the loop.
The candidate marking which ends the structural phase is discarded:
the solver has to propose a new one after the marking equation and transition ordering constraints are added
to begin the behavioral phase.
When $Phi$ becomes unsatisfiable, `petrivet` also returns the solver's UNSAT core,
mapped back to the refinements it consists of,
as a human-readable explanation of why the target marking is unreachable/uncoverable.

The following paragraphs explain the individual refinements in more detail.

*Initially Unmarked Siphons.*
A siphon which is unmarked in the initial marking must remain unmarked in every reachable marking.
`petrivet` computes the maximal such siphon once at the beginning of the CEGAR procedure
(@impl-traps-siphons, with $S = P without supp(M_0)$), and sets every place variable it contains to zero:
$ and.big_(p in D) M(p) = 0. $
This preemptive refinement prevents the solver from placing tokens where they can never be placed.

*Initially Marked Traps.*
A trap which is marked in the initial marking must remain marked in every reachable marking,
so a potential solution marking $M$ which leaves such a trap unmarked is spurious @esparza_verification_2000.
`petrivet` computes the maximal trap $Q$ among the places unmarked in $M$ (@impl-traps-siphons, with $S = P without supp(M)$).
Every trap unmarked in $M$ is contained in $Q$,
so if any initially marked trap is unmarked in $M$,
then $Q$ is initially marked as well.
In that case the SMT problem is refined with the constraint $sum_(p in Q) M(p) >= 1$.

This is the _maximal M-trap_ strategy of @esparza_verification_2000[Sec. 6].
It is cheap to compute, but since every other such trap is contained in $Q$,
it gives the weakest of the constraints that could be derived.
@esparza_smt-based_2014 and @thierry-mieg_structural_2020 instead find such a trap with a separate SAT query,
which can return a smaller trap and a stronger constraint, at the cost of a SAT call per refinement.

*P-invariants.*
Given a potential solution marking $M$, we search for an invariant which separates $M$ from the initial marking,
using the encoding $phi_"kind" and phi_"separate"(M_0, M)$ from @impl-p-invariants.
A solution proves that $M$ is not reachable, so $M$ is a spurious solution of the abstraction.

`petrivet` first searches for a P-invariant ($bowtie = {=}$) following @thierry-mieg_structural_2020.
If none exists, `petrivet` also searches for either a P-subinvariant or P-surinvariant
in a single query ($bowtie = {<=, >=}$).
Exact P-invariants are preferred because they give a stronger refinement,
but any of the three types of invariants can be used to refine the abstraction.
All three variants of $phi_"kind"$ are encoded once, each guarded by a Boolean selector variable,
in a separate solver instance from the main abstraction.
For each candidate $M$, only the invariant type selector and $phi_"separate"(M_0, M)$ are asserted,
and they are retracted again afterwards,
so the same instance is reused for every candidate of a CEGAR run.

If an invariant $y$ is found, the abstraction is refined with the constraint
$ sum_(p in supp(y)) y(p) dot M(p) med bowtie med y dot M_0 $
over the abstraction's place variables $M(p)$, with the same relation $bowtie$ that was used to find $y$.
The constraint holds in every reachable marking but not in the candidate,
so the candidate is excluded from all later iterations,
together with every other spurious marking that violates the same invariant.

*Marking Equation.*
On entering the behavioral phase, we introduce a non-negative integer variable $X(t)$ for each transition $t in T$,
and relate each place variable to the initial marking and the transition variables according to the marking equation:
$ and.big_(p in P) M(p) = M_0(p) + sum_(t in T) C(p, t) dot X(t) $
From this point on, a potential solution from the SMT solver consists of a marking $M$ and a Parikh vector $X$.

*Transition Ordering.*
The marking equation has no notion of the order in which transitions fire.
It therefore admits Parikh vectors containing cycles of transitions which "borrow" tokens that do not exist:
a transition $t$ consumes from a place $p$ which is insufficiently marked initially,
and is repaid by a transition which in turn depends on $t$.
@thierry-mieg_structural_2020 introduces two refinements against this. \
Let $C^-$ and $C^+$ be the pre- and post-incidence matrices, so that $C = C^+ - C^-$,
and let $F(p, t) := {t' in pre(p) without {t} : C(p, t') > 0}$ be the transitions other than $t$
which have a net additive effect on $p$.
The _read arc constraint_ applies to every transition $t$ which reads from an insufficiently marked place,
i.e. $C(p, t) = 0$ and $C^-(p, t) > M_0(p)$:
$ X(t) > 0 => or.big_(t' in F(p, t)) X(t') > 0. $
The _causality constraint_ applies to every insufficiently marked input place $p$ of $t$, i.e. $C^-(p, t) > M_0(p)$,
and additionally requires one of the transitions in $F(p, t)$ to fire _before_ $t$.
For this, every transition gets an integer variable $o(t)$ which models its first occurrence in a firing sequence:
$ X(t) > 0 => or.big_(t' in F(p, t)) (X(t') > 0 and o(t') < o(t)). $
If $F(p, t) = emptyset$, the disjunction is empty and the constraint becomes $X(t) = 0$.

These two refinements are closely related.
The premise of the causality constraint includes read arcs,
and its conclusion implies the conclusion of the read arc constraint,
so every read arc constraint is implied by a causality constraint.
`petrivet` therefore unifies them into a single refinement, called _transition ordering_,
which only uses the causality form, and is restricted to pairs $(p, t)$ with $C^-(p, t) > M_0(p)$
for which there is a risk of spurious "borrowing" solutions.

To find these, `petrivet` computes the dependency graph over $P union T$
with an edge $p -> t$ for any $p in P$ and $t in T$ with $C^-(p, t) > M_0(p)$,
i.e. wherever $p$ is insufficiently marked for $t$ in $M_0$,
and all edges $t -> p$ from the original net.
A borrowing cycle corresponds to a cycle in this dependency graph.
In particular, a read arc appears as a cycle of length two: $p -> t -> p$.
Causality constraints are added for exactly those pairs $(p, t)$ where $p$ and $t$
are part of the same strongly connected component of the dependency graph, and
ordering variables $o(t)$ are only added for exactly those involved transitions.

#lemma(title: "Transition Ordering Correctness")[
  Together with the marking equation, adding the causality constraints for the dependent pairs $(p, t)$
  does not change the set of solutions $(M, X)$.
]

*Proof.*
Let $(p, t)$ be an edge whose endpoints lie in different strongly connected components of the dependency graph,
and let $(M, X)$ be a solution with $X(t) > 0$.
Since $t$ and $p$ are not on a common cycle, $t$ does not produce into $p$,
so $C(p, t) = -C^-(p, t) < -M_0(p)$.
The marking equation and $M(p) >= 0$ give $sum_(t' != t) C(p, t') X(t') > 0$,
so some transition $t' in F(p, t)$ has $X(t') > 0$.
For the ordering, number the strongly connected components in topological order,
and replace every $o(t)$ by its component's number, refined by the old $o(t)$ inside each component.
This keeps all constraints within a component satisfied,
and for every edge $t' -> p -> t$ between different components gives $o(t') < o(t)$. $qed$

Unlike in @thierry-mieg_structural_2020, where the constraints are added step by step after each satisfiable query,
the transition ordering refinement is added once, together with the marking equation,
since it does not depend on any candidate solution.
As noted by @thierry-mieg_structural_2020, the result is still an over-approximation:
the ordering variables only model the first firing of each transition,
and subsequent firings remain subject to the same risk of spurious borrowing.

*Trap Becomes Marked.*

A trap which is unmarked in $M$ but has some transition from $supp(X)$ in its preset is a contradiction:
firing the transitions in $supp(X)$ would have to mark the trap, which then must stay marked.
To search for one, `petrivet` computes the maximal trap $Q$ among the places
which are unmarked in $M$ and adjacent to $U$ (@impl-traps-siphons),
and checks whether any transition in $pre(Q)$ is in $U$.
If so, the SMT problem is refined with the constraint
$ sum_(t in pre(Q)) X(t) > 0 => sum_(p in Q) M(p) > 0. $

This refinement is weaker than it could be.
$Q$ is computed as a trap of the whole net $N$, and the constraint does not depend on $X$;
$X$ only determines which trap is checked.
A stronger variant, suggested by Javier Esparza (personal communication),
considers traps of the subnet $N_U$, which contains all places but only the transitions in $U$.
A trap of $N_U$ only needs to satisfy the trap property for the transitions that are actually fired,
so it need not be a trap of $N$, and there are more candidates for a contradiction.
Since $Q$ may now lose tokens through the transitions $L = post(Q) without pre(Q)$ outside $U$,
the constraint only holds for solutions which fire none of them:
$ sum_(t in L) X(t) = 0 and (M_0(Q) > 0 or sum_(t in pre(Q)) X(t) > 0) => sum_(p in Q) M(p) > 0. $
For a trap of $N$, $L = emptyset$, so the implemented refinement is the special case of this constraint
(with the $M_0(Q) > 0$ case already covered by the Initially Marked Traps refinement).
Implementing the subnet variant only requires restricting the trap computation to the transitions in $U$,
and is left to future work.

*Realizing the Parikh Vector.*
If no refinement excludes the candidate solution $(M, X)$,
`petrivet` attempts to find a firing sequence $sigma$ from $M_0$ which fires each transition $t$ exactly $X(t)$ times,
using a depth-first search over the markings and the remaining budget $X_"budget" (t) = X(t) - X_"fired" (t)$ for each transition $t$.
The depth-first search is also reduced with stubborn sets @valmari_stubborn_1991.
If it succeeds, the sequence ends in $M$ and $M_"target"$ is reachable/coverable.
Otherwise, the search records the dead ends where the least budget remained,
and derives _increment constraints_ from them following @wimmel_applying_2011.
For each group of places $S$ that blocks the remainder of $X_"budget"$ from firing,
it estimates the number of tokens $n$ that the group additionally needs,
and requires the transitions $t in pre(S) without supp(X_"budget")$ to fire often enough:
$ sum_(t : Delta_S (t) > 0) Delta_S (t) dot X(t) >= n + (#[tokens already produced along the dead end]) \
quad "where" Delta_S (t) := sum_(p in S) C(p, t). $

Increment constraints are not facts about the net:
they steer the solver towards a Parikh vector that can be realized,
and are only justified for the minimal solutions of $Phi$, which `petrivet` currently does not enforce.
A proof of unsatisfiability which relies on an increment constraint therefore proves nothing,
and `petrivet` returns an inconclusive result instead (line 13 of @algorithm-cegar).
This is an incomplete implementation of @wimmel_applying_2011, which iterates minimal solutions
and maintains a queue of increment constraints and other so-called _jump constraints_
to be added to the SMT problem.
However, even that more extensive implementation is not a full decision procedure.

*Summary.*
`petrivet`'s CEGAR engine is a semi-decision procedure for reachability and coverability
in both positive and negative cases,
providing a witness firing sequence or a set of contradictory lemmas, respectively.
Between these two outcomes, the solver may run until its deadline or become otherwise overwhelmed
and return an inconclusive result.
This is unavoidable to some extent, considering the enormous computational complexity of the problems at hand.
But the current implementation also leaves room for improvement:
the trap refinement in the behavioral phase does not yet use the subnet induced by the Parikh vector,
increment constraints are not yet restricted to minimal solutions,
and jump constraints are not yet implemented at all.
The effectiveness of the engine is evaluated in @eval-correctness.