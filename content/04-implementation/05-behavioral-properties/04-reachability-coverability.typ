#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *

== Reachability & Coverability <implementation-reachability-coverability>

This section describes the implementation of reachability and coverability together,
as they share a common engine in `petrivet` which is based on the paradigm of counterexample-guided
abstraction refinement (CEGAR) @clarke_counterexample-guided_2000.

The following is a summary of known results for reachability and their implementation status in `petrivet`.

The initial implementation of reachability checking in `petrivet` was merely the RG semi-algorithm
of constructing the reachability graph and checking for the target marking within it.
This is will never terminate in the unreachable case for unbounded nets.

The current implementation is more sophisticated than the reachability graph approach.

=== CEGAR Engine

The engine is based on the paradigm of counterexample-guided abstraction refinement (CEGAR) clarke_counterexample-guided_2000,
a well-known technique in formal verification and model checking that has been applied to Petri nets @wimmel_applying_2011 @hajdu_new_2015 @thierry-mieg_structural_2020.

In general, the paradigm operates by constructing and iteratively refining an overapproximated abstraction of the original system.
If the property of interest does not hold in the abstraction, then by construction it does not hold in the original system either.
If the property holds in the abstraction, then it yields a potential solution for the original system.
Simulating the potential solution in the original system either confirms the solution or produces a counterexample,
which is then used to refine the abstraction, and the process repeats until a definitive answer is found.

#figure(
  image("../../../figures/cegar/diagram.svg"),
  caption: [Counterexample-guided abstraction refinement (CEGAR) paradigm.],
)

CEGAR offers a way to decide properties while largely avoiding the state space explosion problem.
The abstraction can be in the form of an SMT problem definition, and checking the property becomes a satisfiability
check using an SMT solver such as Z3 @de_moura_z3_2008.
Abstraction refinement is performed by adding additional constraints, and sometimes additional variables, to the SMT problem definition.
Simulation, the source of counterexamples and the refinements derived from them, is specific to the type of system being analyzed.

In the case of Petri nets, simulation does not only involve firing transitions.
While a concrete firing sequence from the initial to the target marking is the inevitable final certificate of a positive result,
there are several other (cheaper) sources of counterexamples which can be used to refine the abstraction beforehand
to delay confrontation with the state space explosion problem as long as possible.
By arranging these refinement opportunities in order of increasing computational cost, we can attempt to find a counterexample
to each potential solution as cheaply as possible, and only resort to the expensive simulation of firing transitions when all other options have been exhausted.

`petrivet` implements this CEGAR procedure as a shared engine for checking reachability and coverability properties.
A `CegarProblem` is defined as a four-tuple of Net, initial marking, target marking, and property type (reachability or coverability):
```rust
pub struct CegarProblem<'a> {
    /// The net being analyzed.
    pub net: &'a DenseNet,
    /// The initial marking.
    pub m0: &'a IdxMarking<u32>,
    /// The target marking.
    pub target: &'a IdxMarking<u32>,
    /// The property being checked.
    pub property: CegarProperty,
}

pub enum CegarProperty {
    /// Is the target marking reachable from the initial marking?
    Reachable,
    /// Is some marking which covers the target marking reachable from the initial marking?
    Coverable,
}
```

The only behavioral difference between reachability and coverability is in the initial definition of the SMT problem,
where non-negative integer variables are defined for each place in the net.
In the reachability case, the place variables are constrained to be equal to the target marking,
while in the coverability case, the place variables are constrained to be greater than or equal to the target marking.
All other aspects of the CEGAR engine are identical for both property types.

The following sections describe the ladder of refinements implemented in `petrivet`,
in ascending order of algorithmic complexity.

These are each implemented as a pair of functions: one to search for and potentially yield a counterexample,
and one to use the counterexample to refine the SMT problem definition, eliminating the spurious solution
(and hopefully many others) from the search space.

The first section of refinements are used in the first phase, where the SMT solver only one marking variable per place,
and no knowledge of transitions or the marking equation. In this phase, a potential solution is a marking.

==== Initially Unmarked Siphons

A siphon which is unmarked in the initial marking must remain unmarked in every reachable marking.
Since there is exactly one maximal unmarked siphon in the initial marking, we can apply @best_petri_2024[Algorithm 6.19]
to find it once at the beginning of the CEGAR procedure, and set every place variable it includes in the SMT problem to zero.
This is a preemptive refinement which runs in $O(|P|^2 * |T|^2)$ which prevents the solver from placing tokens where they can never be placed.

==== Initially Marked Traps

A trap which is marked in the initial marking must remain marked in every reachable marking.
A potential solution marking which does not mark an initially marked trap is therefore spurious.
The SMT problem can be refined by adding a constraint that the sum of the place variables
which make up the trap must be greater than zero.

@thierry-mieg_structural_2020 use a separate SAT instance to find initially marked traps,
but I apply the dual of @best_petri_2024[Algorithm 6.19] to find the maximal unmarked trap
in the candidate marking, and then check whether it is marked in the initial marking.

==== P-invariants (P-subinvariants, P-surinvariants)

Given a potential solution marking $M$, we can check for the existence of a P-invariant, P-subinvariant, or P-surinvariant
in the net which is violated by $M$ with respect to the initial marking $M_0$.

// TODO: Formulate the SAT problem
// TODO: Describe how I expand upon Thierry-Mieg's P-invariant approach

==== Marking Equation

In order to encode the marking equation into the SMT problem, we introduce a non-negative integer variable for each transition in the net,
and relate each place variable to the initial marking and the transition variables according to the marking equation:

$ forall p in P: M(p) = M_0(p) + sum_{t in T} C(p, t) * X(t) $

From this point onward, every candidate solution from the SMT solver additionally includes a Parikh vector.
This gives us an important hint about the potential firing sequence which could lead to the candidate solution marking.

==== Transition Ordering

@thierry-mieg_structural_2020 introduce two novel refinement types, Read Arc Constraints and Causality Constraints.

==== Trap Becomes Marked

A candidate marking and Parikh vector pair can be shown to be spurious by exhibiting a trap which is unmarked in the
candidate marking, but which would have to become marked if the transitions in the support of the Parikh vector
were fired.

To search for such a contradiction, we deduce the maximal unmarked trap in the candidate marking among the places
adjacent to the transitions in the support of the Parikh vector, and check whether any of the transitions which
feed tokens into the trap are in the support of the Parikh vector.

To refine the SMT problem, we add an implication that if any of the transitions which feed tokens into the trap are
included in the Parikh vector, then at least one of the places in the trap must be marked:

$ sum_{t in T_"feed"} X(t) > 0 => sum_{p in P_"trap"} M(p) > 0 $
