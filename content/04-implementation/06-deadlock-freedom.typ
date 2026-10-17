
=== Deadlock-Freedom

A Petri net is said to be _deadlock-free_ if, from every reachable marking, there exists at least one enabled transition;
that is, there is no reachable marking from which the system cannot proceed further.

Deadlock-freedom is Ackermann-hard in general, but there are efficient procedures for certain subclasses of nets.

The following is a summary of known results for deadlock-freedom and their implementation status in petrivet.

#figure(
  kind: table,
  caption: [Known results for deadlock-freedom and their implementation status.],
  table(
    columns: (auto, auto, auto, auto, auto, auto),
    [*Applicability*], [*Theorem*], [*Intuition*], [*Complexity*], [*Citation*], [*Implementation status*],

    [State Machine],
    [Deadlock-free iff there exists a marked place such that there is no path from it to any place with an empty postset],
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

petrivet's API provides the following methods for checking deadlock-freedom:

#grid(
  columns: (auto, auto, auto),
  [Method], [Returns], [Description],
  [`pn.is_deadlock_free()`], [`bool`], [Returns true if the Petri net is deadlock-free and false otherwise.],
  [`pn.is_efficiently_deadlock_free()`], [`Option<bool>`], [Returns true if the Petri net is deadlock-free and false otherwise, using only efficient procedures if possible, returning `None` otherwise.],
  [`pn.deadlocks()`], [`Iterator<Deadlock>`], [Return an iterator over all reachable deadlock markings, if any exist.],
)
