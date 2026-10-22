#import "../lib.typ": *

=== Background

==== Petri Nets

Petri nets, model checking, and fundamental concepts. See @murata_petri_1989.
Define important concepts here:
- structure
- tokens and markings
- transition firing semantics
- reachability and coverability, reachability graph, Karp-Miller coverability graph
- boundedness
- deadlock-freedom
- liveness L0 - L4
- subclasses of nets: circuit, state machine, marked graph, free choice, extended free choice, asymmetric choice, general nets
- traps and siphons, Commoner/Hack Criterion

==== Structure

A net $N = (S, T, F)$ is a bipartite directed graph consisting of a finite set of places $S$,
a finite set of transitions $T$ disjoint from $S$, and a flow relation $F ⊆ (S × T) ∪ (T × S)$.
The places and transitions form the nodes of the graph,
and the flow relation defines the directed edges ("arcs") between them.

A _marking_ is a mapping $M: S → ℕ$ that assigns a non-negative integer number of tokens to each place in $S$.

A _Petri net_ $(N, M_0)$ is a net $N$ together with an initial marking $M_0$.

The _preset_ of a transition $t ∈ T$, denoted $•t$, is the set of places $s$ such that $(s, t) ∈ F$ (there is a directed edge from $s$ to $t$).
and the _postset_ of $t$, denoted $t•$, is the set of places $s$ such that $(t, s) ∈ F$ (there is a directed edge from $t$ to $s$).
The preset and postset of a place $s ∈ S$ are defined analogously.
In this thesis, we describe the preset and postset of a transition $t$ as the _input places_ and _output places_ of $t$,
and the preset and postset of a place $s$ as the _input transitions_ and _output transitions_ of $s$.

Let $N = (P, T, F)$ be a net with $n = |P|$ places and let $M_0$ be the initial marking.
For $X subset.eq P$ write $M_0(X) = sum_(p in X) M_0(p)$;
$X$ is _marked_ iff $M_0(X) > 0$.

==== Deadlock-freedom <background-deadlock-freedom>

A Petri net is said to be _deadlock-free_ if every reachable marking enables at least one transition.

==== K-Boundedness <background-boundedness>

A place is said to be _k-bounded_ for some finite integer `k` if the place never
contains more than `k` tokens in any reachable marking.

A place is _bounded_ if it is k-bounded for some finite `k`, and _unbounded_ otherwise.
The Petri net as a whole is said to be (k-)bounded if all of its places are (k-)bounded,
and unbounded if at least one of its places is unbounded.

==== Liveness <background-liveness>

A transition is said to be _live_ if, from every reachable marking, it is possible to eventually fire it again;
that is, there is no reachable marking from which the transition can never be fired again.
A Petri net is said to be live if all of its transitions are live.

We can further specify liveness by classifying it into four levels, L0 through L4 @murata_petri_1989, which I explain here in my own words:
- L0 ("dead"): No firing sequence contains the transition. The transition is not enabled in any reachable marking.
- L1 ("not dead"): There exists a firing sequence containing the transition. The transition is enabled in at least one reachable marking.
- L2 (implies L1): For any finite integer k, there exists a firing sequence containing the transition k times.
- L3 (implies L2): There exists an infinite firing sequence containing the transition infinitely many times. The transition can be fired infinitely often, but there may be reachable markings where this is no longer possible.
- L4 ("live", implies L3): From every reachable marking, there exists a firing sequence containing the transition. The transition can always be fired again.

==== Reachability <background-reachability>

The reachability problem asks whether it is possible to reach a target marking $M$ from the initial marking $M_0$ by firing a sequence of transitions.

==== Coverability <background-coverability>

The coverability problem asks whether it is possible to reach a marking $M'$ from the initial marking $M_0$
such that $M'(p) ≥ M(p)$ for all places $p$ in the net.

==== Structural Subclasses of Petri Nets

Nets can be classified into various subclasses based on their structural layout.
These subclasses imply various behavioral properties, which can be exploited to improve the efficiency of analysis algorithms.

Some notable net subclasses include, in order of increasing specialization:
- General Net (unrestricted):
  - As defined above.
- Asymmetric Choice Net:
  - If two transitions share an input place, then the input places of one transition form a subset of the input places
  of the other transition.
- (Extended) Free Choice Net:
  - Free Choice Net: if two transitions share an input place, then that is their only input place.
  - Extended Free Choice Net: if two transitions share an input place, then they share all input places.
- Marked Graph (also known as T-net):
  - Every place has exactly one input and one output transition.
  - Fundamental property: the sum of tokens in a circuit is invariant under transition firings.
- State Machine:
  - Every transition has exactly one input and one output place.
  - Fundamental property: the sum of tokens is invariant under transition firings.
- Circuit:
  - Every place and every transition has exactly one input and one output.
  - The entire net forms a single directed circuit.

==== Traps and Siphons <background-traps-siphons>

A *trap* is a nonempty $Q subset.eq P$ with $post(Q) subset.eq pre(Q)$.
A *trap* is a set of places such that every transition which consumes a token from a place in the trap also produces
a token in some place in the trap. A trap is _marked_ if at least one place in the trap has a token in the current marking.
Once a trap is marked, it is impossible to completely unmark it through transition firings.
Once marked, forever marked.

Traps

A *siphon* is a nonempty $D subset.eq P$ with $pre(D) subset.eq post(D)$.
A *siphon* is a set of places such that every transition which produces a token in a place in the siphon also consumes
a token from some place in the siphon. A siphon is _unmarked_ if all places in the siphon have zero tokens in the current marking.
Once a siphon is unmarked, it is impossible to mark it again through transition firings.
Once empty, forever empty.

Siphons have a strong conceptual connection to deadlocks.
Indeed, the set of unmarked places in any deadlock marking forms a siphon:
every transition is dead because it has an input place in the siphon, and therefore cannot mark any place in the siphon again.

// TODO: Add figures
