#import "../../lib.typ": *
#import "@preview/lovelace:0.3.0": *

=== Traps and Siphons <impl-traps-siphons>

Many procedures in `petrivet` rely on the siphons and traps of a net, which were introduced in @background-traps-siphons.
Specifically, they need the _maximal_ siphon or trap in a given set of places $S subset.eq P$,
which is unique because siphons and traps are closed under union.
Typically $S$ is the set of unmarked places in some marking $M$, i.e. $S = P without supp(M)$.

Both are computed by the same shrinking fixpoint algorithm @best_petri_2024[Alg. 6.19].
Starting from $S$, the algorithm repeatedly removes places that violate the siphon or trap condition
until no more places can be removed.

- *Siphon:* $p$ is removed if some transition $t in pre(p)$ produces into $p$ but consumes from no place in the current set,
  i.e. $pre(t) inter S = emptyset$. Such a $t$ violates $pre(D) subset.eq post(D)$.
- *Trap:* $p$ is removed if some transition $t in post(p)$ consumes from $p$ but produces into no place in the current set,
  i.e. $post(t) inter S = emptyset$. Such a $t$ violates $post(Q) subset.eq pre(Q)$.

The set only ever shrinks, so the algorithm terminates after at most $|S|$ removals.
The result is a siphon (or trap), since no place violating the property remains,
and it is maximal, since a place belonging to some siphon (or trap) $D subset.eq S$ is never removed.
If the result is empty, $S$ contains no proper siphon (or trap).

Rather than rescanning all places after every removal, `petrivet` keeps a worklist of places to check.
Removing $p$ can only affect places that are connected to $p$ through a transition in the relevant direction:
for siphons, removing $p$ can only invalidate the places $q in post(post(p))$,
since only the transitions in $post(p)$ lose an input place,
so only those places are pushed onto the worklist again.
@algorithm-maximal-siphon shows the siphon version;
the trap version is obtained by swapping every preset and postset.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [The maximal siphon contained in a set of places $S$.],
  pseudocode-list(numbered-title: [`MaximalSiphonIn`$(N, S)$])[
    + $D <- S$; #h(0.5em) $W <- S$
    + *while* $W != emptyset$:
      + $p <- op("pop")(W)$
      + *if* $p in D$ *and* $exists t in pre(p): pre(t) inter D = emptyset$:
        + $D <- D without {p}$
        + *for each* $t in post(p)$, $q in post(t)$ *with* $q in D$: #h(0.3em) $op("push")(W, q)$
    + *return* $D$
  ]
) <algorithm-maximal-siphon>

Sets of places are represented as fixed-size bitsets over the place indices of the net,
allowing membership tests and set operations to be performed in constant time.
Complementing the support of a marking to obtain the initial set $S$ is a single pass over the bitset.

These procedures are used throughout the behavioral analysis.
A dead marking always has an unmarked proper siphon,
so the maximal unmarked siphon is a cheap check for deadlock-freedom (see @impl-deadlock-freedom).
A marked trap stays marked and an unmarked siphon stays unmarked,
so if the maximal trap that is unmarked in a target marking $M$ is marked in $M_0$, then $M$ is unreachable.
This gives a quick unreachability check @esparza_petri_2026[Cor. 4.4.4] and several refinements of the CEGAR engine
(see @impl-reachability-coverability).