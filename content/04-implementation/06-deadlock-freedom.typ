#import "@preview/lovelace:0.3.0": *
#import "../lib.typ": *
#import "../../lib/tum/elements.typ": *

== Deadlock-Freedom

The following is a summary of known results for deadlock-freedom and their implementation status in petrivet.

#figure(
  kind: table,
  caption: [Known results for deadlock-freedom and their implementation status.],
  table(
    columns: (auto, auto, auto, auto, auto, auto),
    [*Applicability*], [*Theorem*], [*Intuition*], [*Complexity*], [*Citation*], [*Implementation status*],

    [General],
    [Deadlock-free <= there exists a transition with an empty preset],
    [
      A transition with an empty preset is vacuously enabled at every marking,
      ensuring that the set of enabled transitions is never empty.
    ],
    [O(|T|)],
    [],
    [Implemented],

    [State Machine],
    [Deadlock-free <=> there exists a marked place with no path to a sink place.],
    [
      (⇐) The set R of places reachable from the marked place p₀ is closed under successors and holds a token forever.
      It contains no place with an empty postset, so some transition is always enabled.
      (⇒) If every marked place reaches a sink, walk all tokens into sinks. That marking is dead.
    ],
    [O(|P| + |T|)],
    [Own derivation], // TODO: Search literature
    [Implemented],


  ),
)

Dimensions:

Operation:
- check whether deadlock free or not -> bool
- enumerate deadlocks -> Iterator<Deadlock>

Scope:
- entire Petri net

petrivet's API provides the following methods for checking deadlock-freedom:

#grid(
  columns: (auto, auto, auto),
  [Method], [Returns], [Description],
  [`pn.is_deadlock_free()`], [`bool`], [Returns true if the Petri net is deadlock-free and false otherwise.],
  [`pn.is_efficiently_deadlock_free()`], [`Option<bool>`], [Returns true if the Petri net is deadlock-free and false otherwise, using only efficient procedures if possible, returning `None` otherwise.],
  [`pn.deadlocks()`], [`Iterator<Deadlock>`], [Return an iterator over all reachable deadlock markings, if any exist.],
)

=== Deadlock Freedom of State Machines

A deadlock in a state machine puts all tokens on sink places (i.e., places with an empty postset).
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

=== Deadlock-Freedom of Marked Graphs

For strongly connected marked graphs, being live, deadlock-free, and having an infinite firing sequence coincide,
and all hold iff there is no unmarked circuit @esparza_petri_2026[Thm. 5.2.8] @murata_petri_1989[Thm. 7].
In general, an unmarked circuit only makes the transitions downstream of it die, while the rest of the net may keep
firing forever. The criterion is therefore that some transition is _not_ downstream of an unmarked circuit.

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
