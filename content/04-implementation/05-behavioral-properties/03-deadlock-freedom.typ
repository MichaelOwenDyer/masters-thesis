#import "@preview/lovelace:0.3.0": *
#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *

== Deadlock-Freedom

This section describes the implementation of the deadlock-freedom property of Petri nets within `petrivet`.
See @background-deadlock-freedom for a discussion of the property itself.

*Scope.* Deadlock-freedom, unlike boundedness and liveness, is a property of the entire Petri net only,
and not of any individual node.

*Granularity.* Deadlock-freedom can be framed as a yes/no question ("is the Petri net deadlock-free?"),
or as a search for all reachable deadlocks.

`petrivet` offers two entry points for checking deadlock-freedom of a `PetriNet`, covering both combinations:

+ `PetriNet::is_deadlock_free() -> Option<bool>` returns true if the Petri net is deadlock-free and false otherwise.
+ `PetriNet::deadlocks() -> Deadlocks` returns an iterator over all reachable deadlock markings, if any exist.

The former supports both cost-control mechanisms, while the latter only supports a timeout.
This is because currently no efficient procedures are implemented for enumerating all reachable deadlocks,
and so the efficiency restriction would always return `None` for this method.
Future work may implement efficient procedures for enumerating deadlocks,
in which case the efficiency restriction could be supported.

=== Procedure Dispatch

The following procedures are utilized for `PetriNet::is_deadlock_free()`, in order of increasing computational cost.

*Initial Marking.*
Trivial: if the initial marking is a deadlock, then the Petri net is not deadlock-free,
and the initial marking is the only reachable deadlock.

*Circuits.*
A circuit is deadlock-free if and only if it has at least one token in its initial marking.

*Source Transitions.*
The existence of a transition with an empty preset is a sufficient condition for deadlock-freedom,
because such a transition is vacuously enabled at every reachable marking @thierry-mieg_structural_2020[Rule 6].

*State Machines.*
A deadlock in a state machine puts all tokens on sink places (places with an empty postset), or has no tokens at all.
A natural consequence of this is that a strongly connected state machine, where every place has a non-empty postset,
will always be deadlock-free if it has at least one token.
A non strongly-connected state machine may have sink places,
but a deadlock is only reachable if all tokens can reach a sink.
Tokens move independently, so a deadlock is reachable iff every marked place can reach a sink.
The algorithm therefore computes the set of places that can reach a sink by a backward search from the sink places,
and reports deadlock-freedom iff some marked place is not among them.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [Deadlock-freedom of a state machine.],
  pseudocode-list(numbered-title: [`DeadlockFreeStateMachine`$(N, M_0)$])[
    + $P_0 <- P pipe op("filter")(p |-> M_0(p) > 0)$
    + $Sigma <- P pipe op("filter")(p |-> post(p) = emptyset)$
    + $B <- "PreStar"(Sigma)$
    + *return* $P_0 subset.eq.not B$
    +
    + *procedure* $"PreStar"(X)$:
      + $W <- op("list")(X)$
      + *while* $W != emptyset$:
        + $p <- op("pop")(W)$
        + *for each* $t in pre(p)$ *and each* $q in pre(t)$:
          + *if* $q in.not X$: $X <- X union {q}$; #h(0.5em) $op("push")(W, q)$
      + *return* $X$
  ]
) <algorithm-state-machine-deadlock-freedom>

A proof of correctness is given in @proof-state-machine-deadlock-freedom.

*Marked Graphs.*
For strongly connected marked graphs, live, deadlock-freedom, and the existence of an infinite firing sequence
perfectly coincide @esparza_petri_2026[Thm. 5.2.8].
All hold iff every circuit is marked in the initial marking @murata_petri_1989[Thm. 7].
For non strongly-connected marked graphs, deadlock-freedom can hold even if liveness does not,
and it only requires that some transition is not downstream of an unmarked circuit.
Because the transitions on an unmarked circuit are dead @murata_petri_1989,
any transition downstream of an unmarked circuit will eventually die too,
limited by the number of tokens the initial marking places on the path from the unmarked circuit to that transition.
But a transition that is not on or downstream of an unmarked circuit is necessarily live @murata_petri_1989,
and so no deadlock marking can ever occur.

The following algorithm implements this reasoning:
we search for a transition which is not on or downstream of an unmarked circuit,
and report deadlock-freedom iff one is found.


#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [Deadlock-freedom of a marked graph.],
  pseudocode-list(numbered-title: [`DeadlockFreeMarkedGraph`$(N, M_0)$])[
    + $G_0 <- G[T union {p in P | M_0(p) = 0}]$
    + $Z <-$ #h(0.3em) $op("SCC")(G_0)$
      $pipe op("filter")(|C| > 1)$
      $pipe op("flat_map")(C |-> C)$
      $pipe op("filter_map")(v |-> v "if" v in T)$
    + $d <- (bot)_(t in T)$; #h(0.5em) $W <- Z$
    + *while* $W != emptyset$:
      + $t <- op("pop")(W)$
      + *if* $not d[t]$:
        + $d[t] <- top$
        + *for each* $p in post(t)$: #h(0.3em) $op("extend")(W, post(p))$
    + #line-label(<algorithm-marked-graph-deadlock-freedom-return>) *return* $exists t in T: not d[t]$
  ]
) <algorithm-marked-graph-deadlock-freedom>

A proof of correctness is given in @proof-marked-graph-deadlock-freedom.

If the procedures so far have not provided an answer and the efficient-only cost-control mechanism has been enabled,
then at this point `petrivet` will give up and return `None`.
Otherwise, it will continue to the expensive fallback procedures.

*Commoner/Hack Criterion.*
The Commoner/Hack criterion is a sufficient condition for deadlock-freedom
of general Petri nets.
If it holds, then the Petri net is deadlock-free, but if it does not hold, we know little to nothing more of use.

*Reachability Graph Exploration.*
As a last resort, `petrivet` can explore the reachability graph of the Petri net,
searching for deadlock markings.
This is a risky procedure, because the current implementation does not ensure that the Petri net is bounded.
As a safer alternative, `petrivet` could explore the coverability graph instead,
halting on the detection of the first unbounded $omega$-marking, similar to the liveness procedure.
Neither is a perfect solution, however, and further work is needed to implement a safe and complete procedure
for deadlock-freedom of general Petri nets.

