#import "../lib.typ": *

== Background

This section provides a brief overview of the fundamental theory of Petri nets.
For further details, I recommend consulting @murata_petri_1989 @esparza_petri_2026 @best_petri_2024.

=== Structure

A *net* $N = (P, T, F)$ is a connected
#footnote[Not all authors require nets to be connected.]
bipartite directed graph consisting of:
- a finite set of *places* $P$,
  #footnote[The set of places is often denoted $S$ in classical literature, from the German word "Stellen". In this thesis, I choose to use $P$ for consistency with the English word "Place".]
- a finite set of *transitions* $T$ disjoint from $S$, and
- a *flow relation* $F ⊆ (P × T) ∪ (T × P)$.

The places (often drawn as circles) and transitions (rectangles or squares) are the nodes of the graph,
and the flow relation defines directed edges (often called arcs) between them.

#figure(
  image("/figures/nets/background/structure.svg"),
  caption: [A net with one place $P_1$, one transition $T_1$, and one arc from $P_1$ to $T_1$.],
) <fig-bg-structure>

The *preset of a place* $p in P$, denoted $•p$, is defined as ${ t in T : (t, s) ∈ F }$ \ (the set of transitions $t$ such that there is a directed edge from $t$ to $p$).

The *postset of a place* $p in P$, denoted $p•$, is defined as ${ t in T : (p, t) ∈ F }$ \ (the set of transitions $t$ such that there is a directed edge from $p$ to $t$).

The *preset of a transition* $t in T$, denoted $•t$, is defined as ${ p in P : (p, t) ∈ F }$ \ (the set of places $p$ such that there is a directed edge from $p$ to $t$).

The *postset of a transition* $t in T$, denoted $t•$, is defined as ${ p in P : (t, s) ∈ F }$ \ (the set of places $p$ such that there is a directed edge from $t$ to $p$).

In this thesis, the terms *input* and *output* are also used to refer to the preset and postset, respectively.

=== Tokens and Markings

A *marking* is a mapping $M: S → ℕ$ that assigns a non-negative integer number of tokens to each place.
Tokens are often drawn as black dots inside the places.

#figure(
  image("/figures/nets/background/token.svg"),
  caption: [A place with one token.],
) <fig-bg-token>

A marking $M$ *covers* another marking $M'$ if $M(p) ≥ M'(p)$ for all $p in P$, that is,
$M$ has at least as many tokens in every place as $M'$.

A place $p in P$ is called *marked* if it contains at least one token, and *unmarked* otherwise.

A set of places $X subset.eq S$ is called marked if at least one place in the set is marked, and unmarked otherwise.

The *support* of a marking $M$, denoted $supp(M)$, is the set of marked places in the marking: ${ p in P : M(p) > 0 }$.

A *Petri net* (or *system*) $(N, M_0)$ is a net $N$ paired with an initial marking $M_0$.

=== Transition Firing Semantics

A transition $t in T$ is *enabled* at a marking $M$ if all of its input places are marked,
i.e. $•t subset.eq supp(M)$.

An enabled transition $t$ may *fire*,
consuming one token from each input place and
producing one token in each output place,
resulting in a new marking $M'$ defined as follows:

$ M'(p) = cases(
  M(p) - 1 "if" p in •t and p in.not t•,
  M(p) + 1 "if" p in t• and p in.not •t,
  M(p) "otherwise"
) $

#figure(
  grid(
    columns: 2,
    column-gutter: 2em,
    row-gutter: 0.6em,
    image("/figures/nets/background/firing_before.svg"), image("/figures/nets/background/firing_after.svg"),
    [(a) Before firing $T_1$], [(b) After firing $T_1$],
  ),
  caption: [
    $T_1$ is enabled because both of its input places are marked.
    Firing it consumes one token from each of $P_1$ and $P_2$ and produces one token in $P_3$.
  ],
) <fig-bg-firing>

A different set of transitions may then be enabled at the new marking $M'$.

A marking is called a *deadlock* if it enables no transitions.

A *firing sequence* is a sequence of transitions $σ = t_1 t_2 ... t_n$.
A marking $M$ enables a firing sequence if $t_1$ is enabled in $M$,
and for each $i = 2, ..., n$, $t_i$ is enabled at the marking resulting from firing $t_1, ..., t_{i-1}$ in order.

The *reachability set* (or reachable markings) of a Petri net $(N, M_0)$ is the set of all markings reachable
from the initial marking $M_0$ by firing some sequence of transitions.

=== Behavioral Properties

Behavioral properties of Petri nets are properties that emerge through the firing of transitions
and the movement of tokens through the net.

==== (K-)Boundedness <background-boundedness>

The boundedness problem asks whether the reachability set of a Petri net is finite.
This is the case if and only if there exists a finite integer `k`
such that no reachable marking contains more than `k` tokens in any place.
A place $p in P$ is said to be k-bounded iff $M(p) ≤ k$ for all reachable markings $M$,
and a Petri net is k-bounded iff all of its places are k-bounded.
If any place is not k-bounded for any finite `k`, it is said to be unbounded,
and the Petri net as a whole is unbounded.

#figure(
  image("/figures/nets/background/unbounded.svg"),
  caption: [An unbounded Petri net: $T_1$ has no input places, so it is always enabled, and every firing adds a token to $P_1$.],
) <fig-bg-unbounded>

==== Reachability <background-reachability>

The reachability problem asks whether there exists an enabled firing sequence $σ$
that leads from the initial marking $M_0$ to a given target marking $M$.

#figure(
  grid(
    columns: 2,
    column-gutter: 2em,
    row-gutter: 0.6em,
    image("/figures/nets/background/reach_yes.svg"), image("/figures/nets/background/reach_no.svg"),
    [(a) Reachable target], [(b) Unreachable target],
  ),
  caption: [
    The net of @fig-bg-firing with its initial marking (filled tokens) and a target marking (outlined tokens). \
    The target in (a) is reached by firing $T_1$ once.
    The target in (b) is unreachable: it requires firing $T_1$ twice, but $P_2$ only has one token.
  ],
) <fig-bg-reachability>

Reachability was shown to be decidable independently by Ernst W. Mayr @mayr_algorithm_1981 (1981) and
S. R. Kosaraju @kosaraju_decidability_1982 (1982),
but an upper bound for its complexity was not established until 2020 by Czerwinski, Lasota, Lazić, and Leroux
@czerwinski_reachability_2020, who showed that it is non-elementary
#footnote[Computational effort grows _faster_ than a tower of exponentials $n^n^n^n^...$ of height proportional to the size of the input.]
and Ackermann-complete
#footnote[https://en.wikipedia.org/wiki/Ackermann_function].
It is one of the most difficult decidable problems in computer science, sitting at the frontier of what is known to be computable.

The naive approach to the reachability problem is to construct the *reachability graph* of the Petri net:
fire all enabled transitions at the initial marking, then fire all enabled transitions at the resulting markings,
and so on, until the target marking is reached, or until all reachable markings have been explored.
Unfortunately, this approach has a fatal flaw: an unbounded Petri net has infinitely many reachable markings,
so the procedure may not terminate.
And even for a bounded Petri net the size of the graph may be astronomically large,
making the procedure intractable for all but the smallest systems.
However, if one does manage to construct the reachability graph, it provides full information about
the dynamic behavior of the Petri net and can be used to decide any property of interest.
The reachability graph is also called the *state space* of the Petri net.

==== Coverability <background-coverability>

The coverability problem asks whether there exists an enabled firing sequence $σ$
that leads from the initial marking $M_0$ to some marking $M'$ which _covers_ a given target marking $M$.

This is a relaxation of the reachability problem, shown to be decidable by Karp and Miller (1969) @karp_parallel_1969
and EXPSPACE-complete by Rackoff (1978) @rackoff_covering_1978.

The analog of the reachability graph for the coverability problem is the *coverability graph* (or Karp-Miller graph).
The coverability graph is always finite, even for unbounded Petri nets, because it uses a special symbol $omega$
to represent an unbounded number of tokens in a place, rather than hopelessly enumerating infinitely many markings.
By doing so however, it loses some information about the exact number of tokens in unbounded places.
The coverability graph can be used to decide coverability and boundedness, but not reachability, liveness,
or deadlock-freedom.

In the bounded case, the coverability graph contains no $omega$ and is identical to the reachability graph.
In the unbounded case, despite the omission of infinitely many markings, the coverability graph
#footnote[
  Technically, for unbounded systems, there are infinitely many possible coverability graphs,
  depending on the method of construction. "The" coverability graph is therefore not a well-defined concept,
  but rather a family of graphs with the same properties.
]
can still be astronomically large, and its construction may be prohibitively expensive
#footnote[
  In fact, the extra overhead of checking for $omega$-markings makes constructing the coverability graph
  even more expensive than constructing the reachability graph in the bounded case.
]
for all but the smallest systems.

==== Liveness <background-liveness>

A transition is said to be *live* iff every reachable marking enables a firing sequence containing that transition,
that is, there is always a way to fire the transition again, no matter what has happened in the past.
A Petri net is said to be live iff all of its transitions are live.

Liveness can be further classified into four levels, #lv(0) through #lv(4) @murata_petri_1989, which I summarize here:

- $lv(0)$ ("dead"): No firing sequence contains the transition. The transition is not enabled in any reachable marking.
- $lv(1)$ ("not dead"): There exists a firing sequence containing the transition. The transition is enabled in at least
  one reachable marking.
- $lv(2) supset.eq lv(1)$: For any finite integer $k$, there exists a firing sequence containing the transition $k$ times.
- $lv(3) supset.eq lv(2)$: There exists an infinite firing sequence containing the transition infinitely many times
  (there may also be reachable markings where this is no longer possible).
- $lv(4) supset.eq lv(3)$ ("live"): From every reachable marking, there exists a firing sequence containing the transition.
  The transition can always be fired again.

A transition is called *strictly* #lv[k]-live if it is #lv[k]-live but not #lv[k+1]-live. \
A Petri net is #lv[k]-live if all of its transitions are #lv[k]-live, and strictly #lv[k]-live if it is #lv[k]-live
but not #lv[k+1]-live.

#figure(
  image("/figures/nets/background/liveness.svg"),
  caption: [
    A Petri net in which each transition $T_k$ is strictly #lv[k]-live.
    $T_0$ is dead because $P_0$ is never marked.
    $T_1$ can fire at most once, since it consumes the only token in $P_1$.
    $T_3$ can fire indefinitely, but only until $T_1$ fires, adding a token to $P_2$ each time.
    $T_2$ can then fire once for each of those tokens, so any finite number of times, but not infinitely often.
    $T_4$ can always eventually fire in every reachable marking, so it is live.
  ],
) <fig-bg-liveness>

==== Deadlock-freedom <background-deadlock-freedom>

A Petri net is said to be *deadlock-free* if every reachable marking enables at least one transition.

Note that deadlock-freedom is a weaker property than liveness: a live Petri net is necessarily deadlock-free,
but a deadlock-free Petri net may not be live (there may be transitions which are never enabled again
after some reachable marking).

Despite this, reachability, liveness, and deadlock-freedom are all reducible to one another (recursively equivalent) @hack_recursive_1974 @hack_decidability_1976:
For any pair of these properties, a Petri net can be transformed in polynomial time into another Petri net
such that the first net has the first property if and only if the second net has the second property.
For this reason, they all share the same Ackermann-complete complexity class.

=== Structural Properties

Structural properties relate to the structure of the net itself, irrespective of the marking.
Despite this, they can hold strong implications for the behavior of the net,
and are often used to prove behavioral properties.

==== Traps and Siphons <background-traps-siphons>

*Traps.*
A trap is a set of places $Q subset.eq S$ with $post(Q) subset.eq pre(Q)$.
Every transition which consumes a token from a place in the trap also produces a token to some place in the trap.
Once a trap becomes marked, it is impossible to completely unmark it through transition firings.
Once marked, forever marked.

*Siphons.*
A *siphon* is a nonempty $D subset.eq S$ with $pre(D) subset.eq post(D)$.
A *siphon* is a set of places such that every transition which produces a token in a place in the siphon also consumes
a token from some place in the siphon. A siphon is _unmarked_ if all places in the siphon have zero tokens in the current marking.
Once a siphon is unmarked, it is impossible to mark it again through transition firings.
Once empty, forever empty.

Siphons have a strong conceptual connection to deadlocks.
Indeed, the set of unmarked places in any deadlock marking forms a siphon:
every transition is dead because it has an input place in the siphon, and therefore cannot mark any place in the siphon again.

#figure(
  image("/figures/nets/background/trap.svg"),
  caption: [The places ${P_1, P_2}$ (shaded) form a trap: every transition that consumes from them ($T_1$, $T_2$) also produces into them. It is unmarked initially, but once $T_3$ fires it stays marked forever.],
) <fig-bg-trap>
#figure(
  image("/figures/nets/background/siphon.svg"),
  caption: [The places ${P_1, P_2}$ (shaded) form a siphon: every transition that produces into them ($T_1$, $T_2$) also consumes from them. It is marked initially, but once $T_3$ fires it stays unmarked forever.],
) <fig-bg-siphon>

*The Siphon-Trap Property, or Commoner-Hack Criterion (CHC).*
A Petri net satisfies the *siphon-trap property* (or *Commoner-Hack Criterion*, CHC) if every siphon contains a marked trap.
This is a characteristic which yields powerful insights into the behavior of even the most general Petri nets.
However, deciding whether the CHC holds is a co-NP-complete problem.
The number of minimal siphons in a net scales exponentially with the number of places,
making the naive approach of enumerating all siphons and checking for a marked trap intractable for large nets.
But the problem of finding a counterexample (a siphon without a marked trap) maps directly to the Boolean satisfiability
problem (SAT), which allows us to leverage the power of modern constraint solvers.

#figure(
  grid(
    row-gutter: 0.6em,
    image("/figures/nets/background/chc_yes.svg"), [(a) The trap is marked],
    image("/figures/nets/background/chc_no.svg"), [(b) The trap is unmarked],
  ),
  caption: [
    The siphon ${P_1, P_2}$ (light) contains the trap ${P_2}$ (dark), and no other trap.
    In (a) the trap is marked, so the siphon can never become empty.
    In (b) the trap is unmarked, and firing $T_3$ empties the siphon forever.
  ],
) <fig-bg-chc>

==== Structural Subclasses  <background-structural-classes>

The structure of a net can be classified into various subclasses,
which are defined by restrictions on the flow relation.
These subclasses are important because they allow for more efficient analysis of many behavioral properties.

Some notable net subclasses include, in order of increasing generality:

*Circuits.*
A net $N = (S, T, F)$ is called a *circuit* if \
$forall p in P: |•p| = |p•| = 1$ and $forall t in T: |•t| = |t•| = 1$. \
Intuitively: the entire net forms a single directed cycle.
The behavioral properties of circuits are trivial.
They inherit the properties of state machines and marked graphs, discussed next.

#figure(
  image("/figures/nets/background/circuit.svg"),
  caption: [A circuit with two places and two transitions.],
) <fig-bg-circuit>

*State Machines.*
A net $N = (S, T, F)$ is called a *state machine* (or *S-net*) if \
$forall t in T: |•t| = |t•| = 1$. \
Intuitively: every transition has exactly one input and one output place.

The behavioral properties of state machines are also trivial.
Most notably, the number of tokens in a state machine is invariant under transition firing: \
$M(S) = M_0(S)$ for every reachable marking $M$.
This is a special case of boundedness called *conservation*.
Additionally, reachability, coverability, liveness, and deadlock-freedom all have linear-time algorithms
in state machines @murata_petri_1989.

#figure(
  image("/figures/nets/background/sm_block.svg"),
  caption: [The building block of state machines: a transition with one input and one output place.],
) <fig-bg-sm-block>
#figure(
  image("/figures/nets/background/sm.svg"),
  caption: [A state machine which is not a circuit: the left place has two output transitions and the right place has two input transitions.],
) <fig-bg-sm>

*Marked Graphs.*
A net $N = (S, T, F)$ is called a *marked graph* (or *T-net*) if \
$forall p in P: |•p| = |p•| = 1$. \
Intuitively: every place has exactly one input and one output transition.

Marked graphs are the structural dual of state machines,
however they are not quite as trivial to analyze.
The fundamental property of marked graphs is that the sum of tokens in a contained _circuit_
is invariant under transition firings, where a circuit is a directed cycle in the net.
An unmarked circuit can never become marked, making the transitions in that circuit dead (#lv(0)-live).
A place in a marked graph is bounded if and only if it is contained in a circuit.
Boundedness, liveness and deadlock-freedom can be decided in linear time in marked graphs.
K-boundedness (finding the exact bound), reachability and coverability are also decidable in polynomial time,
though the algorithms are more complex @murata_petri_1989.

#figure(
  image("/figures/nets/background/mg_block.svg"),
  caption: [The building block of marked graphs: a place with one input and one output transition.],
) <fig-bg-mg-block>
#figure(
  image("/figures/nets/background/mg.svg"),
  caption: [A marked graph which is not a circuit: the left transition has two output places and the right transition has two input places.],
) <fig-bg-mg>

*(Extended) Free Choice Nets.*
A net $N = (S, T, F)$ is called *free-choice* if \
$forall t_1, t_2 in T: pre(t_1) inter pre(t_2) != emptyset => |pre(t_1)| = |pre(t_2)| = 1$. \
Intuitively: if two transitions share an input place, then that is their only input place.

#figure(
  image("/figures/nets/background/fc.svg"),
  caption: [A free-choice net which is neither a state machine nor a marked graph: the left place has two output transitions, and the upper transition has two output places.],
) <fig-bg-fc>

A net $N = (S, T, F)$ is called *extended free-choice* if \
$forall t_1, t_2 in T: pre(t_1) inter pre(t_2) != emptyset => pre(t_1) = pre(t_2)$. \
Intuitively: if two transitions share an input place, then they share all input places.

#figure(
  image("/figures/nets/background/efc.svg"),
  caption: [An extended free-choice net which is not an ordinary free-choice net: the two transitions share both of their input places.],
) <fig-bg-efc>

An fundamental consequence of these properties is that if a set of transitions shares any input places
(so-called "confusion"), then no marking can enable one transition without also enabling the others.

This has powerful analytical implications.
For instance, the Commoner/Hack Criterion (CHC) is both necessary and sufficient for liveness in free-choice nets
@hack_analysis_1972,
and deciding whether a FCN is simultaneously live and bounded (properties which are often desirable together)
can be done in polynomial time @esparza_synthesis_1990 @kemper_efficient_1992.

For purposes of analysis, free-choice nets and extended free-choice nets are equivalent.
Any extended free-choice net can be transformed into a behaviorally equivalent free-choice net through a polynomial-time
transformation @best_petri_2024.
They are listed separately for historical reasons, as the free-choice property was first defined by Michel Hack (1972)
@hack_analysis_1972, and the extended free-choice generalization was only introduced later @best_structure_1987
@esparza_analysis_1991 @desel_free_1995.

In modern Petri net literature, the term "free-choice" now almost universally refers to the extended free-choice property,
and the original free-choice property is now often called "ordinary" or "simple" free-choice.

*Asymmetric Choice Nets.*
A net $N = (S, T, F)$ is an asymmetric choice net if $forall t_1, t_2 in T:
pre(t_1) inter pre(t_2) != emptyset => pre(t_1) subset.eq pre(t_2) or pre(t_2) subset.eq pre(t_1)$. \
Intuitively: if two transitions share an input place, then the input places of one transition
are a subset of the input places of the other transition.

The asymmetric choice property is only a marginal restriction on general nets,
and is not as analytically useful as the free-choice property.
The only notable result known to the author is that the Commoner/Hack Criterion is
sufficient (but not necessary) for liveness in asymmetric choice nets @murata_petri_1989.

#figure(
  image("/figures/nets/background/ac.svg"),
  caption: [An asymmetric choice net which is not an extended free-choice net: the input places of the upper transition are a proper subset of those of the lower transition.],
) <fig-bg-ac>

#figure(
  image("/figures/nets/background/general.svg"),
  caption: [A general net which is not an asymmetric choice net: the two transitions share the middle input place, but neither preset contains the other.],
) <fig-bg-general>

#figure(
  image("/figures/nets/background/classes.svg"),
  caption: [
    The structural subclasses of nets within the scope of this thesis.
  ],
) <fig-bg-classes>
