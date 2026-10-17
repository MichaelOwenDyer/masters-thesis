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

A *trap* is a nonempty $Q subset.eq P$ with $post(Q) subset.eq pre(Q)$.
A *trap* is a set of places such that every transition which consumes a token from a place in the trap also produces
a token in some place in the trap. A trap is _marked_ if at least one place in the trap has a token in the current marking.
Once a trap is marked, it is impossible to completely unmark it through transition firings.
Once marked, forever marked.

A *siphon* is a nonempty $D subset.eq P$ with $pre(D) subset.eq post(D)$.
A *siphon* is a set of places such that every transition which produces a token in a place in the siphon also consumes
a token from some place in the siphon. A siphon is _unmarked_ if all places in the siphon have zero tokens in the current marking.
Once a siphon is unmarked, it is impossible to mark it again through transition firings.
Once empty, forever empty.

==== Model Checking

Model checking is a formal verification technique that systematically explores the state space of a system to verify
whether certain properties hold. It involves constructing a model of the system, specifying properties in a formal
language (such as temporal logic), and using algorithms to check if the model satisfies these properties.
Model checking can be applied to various types of systems, including hardware circuits, software programs, and
concurrent systems modeled by Petri nets.

Mention the breadth of model checking and its subtopics:
- LTL and CTL formula checking
- Partial Order Reduction
- Binary Decision Diagrams / Hierarchical Set Decision Diagrams for state space representation
Many of these topics are in the scope of future work for petrivet.
Within the scope of this thesis, we will focus on the fundamental theory of Petri nets and the implementation
of a library that provides a foundation for future work in these areas.

==== Rust Programming Language