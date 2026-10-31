#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *

=== Reachability and Coverability <implementation-reachability-coverability>

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

In early versions of `petrivet`, the only approach was simply to explore the reachability graph breadth-first
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

The abstraction can be in the form of an SMT problem definition, and checking the property becomes a satisfiability
check using a SAT or satisfiability modulo theories (SMT) solver such as Z3 @de_moura_z3_2008.
Abstraction refinement is performed by adding additional constraints, and sometimes additional variables, to the SMT problem definition.
Simulation, the source of counterexamples and the refinements derived from them, is specific to the type of system being analyzed.

In the case of Petri nets, simulation does not only involve firing transitions.
While a concrete firing sequence from the initial to the target marking is the inevitable final certificate of a positive result,
there are several other (cheaper) sources of counterexamples which can be used to refine the abstraction beforehand
to delay confrontation with the state space explosion problem as long as possible.
By arranging these refinement opportunities in order of increasing computational cost, we can attempt to find a counterexample
to each potential solution as cheaply as possible, and only resort to the expensive simulation of firing transitions when all other options have been exhausted.

`petrivet` implements this CEGAR procedure as a shared engine for checking reachability and coverability properties.
A `CegarProblem` is defined as a four-tuple of Net, initial marking, target marking, and property type (reachability or coverability).
It also accepts an optional callback function to report progress,
and an optional timeout parameter to limit the maximum duration of the analysis

The only behavioral difference between reachability and coverability is in the initial definition of the SMT problem,
where non-negative integer variables are defined for each place in the net.
In the reachability case, the place variables are constrained to be equal to the target marking,
while in the coverability case, the place variables are constrained to be greater than or equal to the target marking.
All other aspects of the CEGAR engine are identical for both property types.

The following sections describe the ladder of refinements implemented in `petrivet`,
in ascending order of algorithmic complexity.

Most of these are implemented as a pair of functions:
one to search for and potentially yield a counterexample,
and one to use the counterexample to refine the SMT problem definition,
eliminating the spurious solution (and hopefully many others) from the search space.
Others are implemented as a one-off refinement which is applied without any counterexample.

The first section of refinements are used in the first phase, where the SMT solver only one marking variable per place,
and no knowledge of transitions or the marking equation. In this phase, a potential solution is a marking.

*Initially Unmarked Siphons.*
A siphon which is unmarked in the initial marking must remain unmarked in every reachable marking.
Since there is exactly one maximal unmarked siphon in the initial marking, we can apply @best_petri_2024[Algorithm 6.19]
to find it once at the beginning of the CEGAR procedure, and set every place variable it includes in the SMT problem to zero.
This is a preemptive refinement which runs in $O(|P|^2 * |T|^2)$ which prevents the solver from placing tokens where they can never be placed.

*Initially Marked Traps.*

A trap which is marked in the initial marking must remain marked in every reachable marking.
A potential solution marking which does not mark an initially marked trap is therefore spurious.
The SMT problem can be refined by adding a constraint that the sum of the place variables
which make up the trap must be greater than zero.

@thierry-mieg_structural_2020 use a separate SAT instance to find initially marked traps,
but I apply the dual of @best_petri_2024[Algorithm 6.19] to find the maximal unmarked trap
in the candidate marking, and then check whether it is marked in the initial marking.

*P-invariants, P-subinvariants, P-surinvariants.*

Given a potential solution marking $M$, we can check for the existence of a P-invariant, P-subinvariant, or P-surinvariant
in the net which is violated by $M$ with respect to the initial marking $M_0$.

// TODO: Formulate the SAT problem
// TODO: Describe how I expand upon Thierry-Mieg's P-invariant approach

*Marking Equation.*

In order to encode the marking equation into the SMT problem, we introduce a non-negative integer variable for each transition in the net,
and relate each place variable to the initial marking and the transition variables according to the marking equation:

$ forall p in P: M(p) = M_0(p) + sum_{t in T} C(p, t) * X(t) $

From this point onward, every candidate solution from the SMT solver additionally includes a Parikh vector.
This gives us an important hint about the potential firing sequence which could lead to the candidate solution marking.

*Transition Ordering.*

@thierry-mieg_structural_2020 introduce two novel refinement types, Read Arc Constraints and Causality Constraints.

*Trap Becomes Marked.*

A candidate marking and Parikh vector pair can be shown to be spurious by exhibiting a trap which is unmarked in the
candidate marking, but which would have to become marked if the transitions in the support of the Parikh vector
were fired.

To search for such a contradiction, we deduce the maximal unmarked trap in the candidate marking among the places
adjacent to the transitions in the support of the Parikh vector, and check whether any of the transitions which
feed tokens into the trap are in the support of the Parikh vector.

To refine the SMT problem, we add an implication that if any of the transitions which feed tokens into the trap are
included in the Parikh vector, then at least one of the places in the trap must be marked:

$ sum_{t in T_"feed"} X(t) > 0 => sum_{p in P_"trap"} M(p) > 0 $
