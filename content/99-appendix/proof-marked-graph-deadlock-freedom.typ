#import "../lib.typ": *
#import "../../lib/tum/elements.typ": *

== Proof of Correctness of @algorithm-marked-graph-deadlock-freedom <proof-marked-graph-deadlock-freedom>

The proof reuses two results of the liveness section for marked graphs, stated there as parts (3) and (4) of the Classification lemma:

+ If $t in D_(>=)$, there is a bound $B_t$ such that $t$ occurs at most $B_t$ times in every firing sequence.
+ If $t in.not D_(>=)$, then $t$ is live.

#lemma(title: "Deadlock-freedom and liveness")[
  $(N, M_0)$ is deadlock-free iff some transition is live.

  _Proof._
  ($arrow.l.double$) Let $t$ be live and suppose a deadlock $M$ were reachable.
  Then $M$ enables no transition, so $[M chevron.r = {M}$ and $t$ is dead at $M$, contradicting liveness.
  ($arrow.r.double$) Suppose no transition is live.
  By Fact 2 every transition lies in $D_(>=)$, so by Fact 1 every firing sequence has length at most $sum_t B_t$.
  Take a firing sequence of maximal length.
  If its final marking enabled a transition, the sequence could be extended, so the final marking is a reachable deadlock.
]

#lemma(title: "Downstream search")[
  After the loop, $d[t] = top$ iff $t in D_(>=)$.
  The loop runs in time $O(|P| + |T|)$.

  _Proof._

  _Soundness._ Every transition put on $W$ is in $Z$ or is the consumer $v_p$ of a place $p$ produced by a transition already marked in $d$.
  So $d[t] = top$ implies $z reach t$ for some $z in Z$.

  _Completeness._ Let $z reach t$ via $z = t_0, p_1, t_1, dots, p_k, t_k = t$ with $z in Z$.
  Every element of $Z$ is on $W$ initially, and when a transition is marked in $d$ all consumers of its output places are pushed.
  Inductively $t_i$ is pushed, and popped, hence $d[t_i] = top$, for all $i$.

  _Complexity._ A transition is expanded only when it is popped with $d[t] = bot$, so at most once, and its expansion pushes $|post(p)| = 1$ entry per output place.
  So at most $|T| + |F|$ entries are ever pushed and popped, and since every place of a marked graph has exactly one input arc and one output arc, $|F| = 2|P|$.
]

#theorem(title: "Correctness")[
  The algorithm returns true iff $(N, M_0)$ is deadlock-free.
  It runs in time $O(|P| + |T|)$.

  _Proof._ By the Deadlock-freedom and liveness lemma, $(N, M_0)$ is deadlock-free iff some transition is live.
  By Facts 1 and 2 a transition is live iff $t in.not D_(>=)$: if $t in D_(>=)$, it fires at most $B_t$ times and is not live, and otherwise it is live.
  By the Downstream search lemma @algorithm-marked-graph-deadlock-freedom-return returns true iff $D_(>=) != T$.
  The set $Z$ is computed in linear time by the Unmarked circuits lemma of the liveness section and Tarjan's algorithm @tarjan_depth-first_1972.
]

#corollary[
  If $N$ is strongly connected, the algorithm returns true iff $N$ has no unmarked circuit.
  If $N$ has a source transition (a transition with an empty preset), the algorithm returns true.

  _Proof._ In a strongly connected net every transition is reachable from every circuit, so $D_(>=) = T$ iff $Z != emptyset$.
  A source transition has no predecessors, so it is not in $D_(>=)$.
]