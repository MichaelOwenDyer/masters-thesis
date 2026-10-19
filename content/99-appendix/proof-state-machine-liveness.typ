#import "../lib.typ": *
#import "../../lib/tum/elements.typ": *

== Proof of Correctness of @algorithm-state-machine-liveness <proof-state-machine-liveness>

The proof uses the following known facts.

+ _Token conservation_ @esparza_petri_2026[Prop. 5.1.2]: $M(P) = M_0(P)$ for every reachable marking $M$.
+ _Token movement_ @esparza_petri_2026[Thm. 5.1.3]: if $q$ is marked and $q reach r$, the transitions on the path can be fired in order, which moves one token from $q$ to $r$ and leaves the other tokens in place.
+ _Strongly connected components_ can be computed in linear time, and $cal(C)$ is acyclic @tarjan_depth-first_1972. Hence every node of $G$ reaches some terminal SCC.

We characterise the levels by the following sets. Let $t in T$ and $A_t := bw(s_t)$.

#lemma(title: "Classification")[
  + _Dead._ $t$ is dead iff no place marked in $M_0$ reaches $s_t$.
  + _Strictly $lv(1)$._ If some place of $P_0$ reaches $s_t$ and $K(t)$ is trivial, then $t$ is $lv(1)$-live and not $lv(2)$-live.
  + _$lv(3)$._ If some place of $P_0$ reaches $s_t$ and $K(t)$ is nontrivial, then $t$ is $lv(3)$-live (hence $lv(2)$-live).
  + _Live._ $t$ is live iff there is a $q in P_0$ with $fw(q) subset.eq A_t$.
  _Proof._
  (1) If $q in P_0$ reaches $s_t$, move a token to $s_t$ (Fact 2) and fire $t$.
  Otherwise, every marking $M$ reachable from $M_0$ satisfies $M(r) > 0 arrow.r.double exists q in P_0: q reach r$, because a firing moves a token only to a successor of its place.
  So $s_t$ is never marked.
  (2) By (1), $t$ is $lv(1)$-live.
  A token's trajectory is a walk in $G$.
  If it fired $t$ twice, the walk between the two firings would close a circuit through $t$, contradicting that $K(t)$ is trivial.
  Hence every token fires $t$ at most once, and by Fact 1 there are $M_0(P)$ tokens, so $t$ fires at most $M_0(P)$ times in any firing sequence.
  (3) Move a token to $s_t$.
  As $K(t)$ is strongly connected and nontrivial, it contains a circuit through $t$, and the token can run around this circuit forever.
  This is an infinite firing sequence containing $t$ infinitely often.
  (4) ($arrow.l.double$) Let $B := fw(q)$ with $B subset.eq A_t$.
  $B$ is closed under successors, so no firing removes the last token from $B$: a firing either does not touch $B$ or moves a token inside $B$.
  Hence $M(B) >= M_0(q) >= 1$ in every reachable $M$.
  Every place of $B subset.eq A_t$ reaches $s_t$, so $t$ can fire from $M$ (Fact 2).
  ($arrow.r.double$) Suppose no such $q$ exists, so for every $q in P_0$ there is $r_q in fw(q) without A_t$.
  By Fact 2 we move every token of every marked place $q$ to $r_q$.
  The resulting marking $M$ has no token in $A_t$.
  $P without A_t$ is closed under successors, since a place with a successor in $A_t$ would itself lie in $A_t$.
  So $s_t$ is never marked from $M$ on, and $t$ is dead at $M$, hence not live.
]

The last case is the generalisation of the argument for (⇒) in @esparza_petri_2026[Thm. 5.1.3].
It remains to translate "$fw(q) subset.eq A_t$" into a property of $cal(C)$.

#lemma(title: "Live SCCs")[
  Let $t in T$ with $K := K(t)$ nontrivial.
  There is a $q in P_0$ with $fw(q) subset.eq A_t$ iff $K$ is terminal and there is a $q in P_0$ such that $K$ is the only terminal SCC reachable from $q$.
  _Proof._
  ($arrow.r.double$) Since $q in fw(q) subset.eq A_t$, $q$ reaches $s_t in K$.
  If an arc left $K$ to a node $r$, then $r$ is reachable from $q$ (through $K$), so $r in fw(q) subset.eq A_t$ reaches $s_t$, and $r in K$, a contradiction.
  Hence $K$ is terminal.
  Let $T'$ be a terminal SCC reachable from $q$.
  Its places lie in $fw(q) subset.eq A_t$, so they reach $s_t$; as no arc leaves $T'$, $s_t in T'$ and $T' = K$.
  ($arrow.l.double$) Let $r in fw(q)$.
  By Fact 3, $r$ reaches a terminal SCC, which is reachable from $q$ and hence equals $K$.
  $K$ is strongly connected, so $r$ reaches $s_t$, i.e. $r in A_t$.
]

#lemma(title: "The algorithm")[
  After termination, for every SCC $K$:
  $ell[K] = lv(0)$ if no node of $K$ is reachable from $P_0$;
  $ell[K] = lv(1)$ if $K$ is trivial and reachable from $P_0$;
  $ell[K] = lv(4)$ if $K$ is nontrivial, terminal, and the unique terminal SCC reachable from some marked place;
  and $ell[K] = lv(3)$ otherwise.
  The algorithm runs in time $O(|P| + |T| + |F|)$.
  _Proof._
  _Reachability._ We show by induction on the position of $K$ in $pi$ that $r[K] = top$ at the end iff some SCC containing a marked place reaches $K$ in $cal(C)$.
  Initially $r = m$.
  Every predecessor of $K$ precedes $K$ in $pi$, so by the induction hypothesis its flag is final when it is visited, and a reachable predecessor sets $r[K]$.
  Conversely only a reachable predecessor or $m[K]$ can set $r[K]$.
  Since the nodes reachable from a marked place are those of the SCCs reachable from its SCC, $r$ identifies exactly the SCCs reachable from $P_0$.

  _Terminal SCCs._ Let $"Term"(K)$ be the set of terminal SCCs reachable from $K$; it is nonempty by Fact 3.
  We show by induction over $"rev"(pi)$ that $sigma[K] = "One"(K')$ if $"Term"(K) = {K'}$ and $sigma[K] = "Multiple"$ otherwise.
  If $K$ is terminal, $"Term"(K) = {K}$.
  Otherwise every path from $K$ leaves through a successor, so $"Term"(K) = union.big_(K' in "succ"(K)) "Term"(K')$.
  All successors follow $K$ in $pi$, so their $sigma$ values are final, and the join of singletons $\{a\}$ and $\{b\}$ describes a singleton iff $a = b$, while $"Multiple"$ is absorbing.
  So the union is a singleton iff all successors agree on the same single terminal SCC.

  _Levels._ Marked SCCs and marked places have the same reachable set, so $K' in Sigma$ iff $K'$ is the unique terminal SCC reachable from some marked place.
  Such a $K'$ is reachable.
  The `match` assigns $lv(0)$ to unreachable SCCs, $lv(1)$ to reachable trivial ones, $lv(4)$ to reachable nontrivial ones in $Sigma$, and $lv(3)$ to the remaining reachable nontrivial ones.
  Since the levels are assigned by a single case distinction, no assignment can overwrite another.

  _Complexity._ The condensation and a topological order are computed in time linear in $G$ @tarjan_depth-first_1972.
  $cal(C)$ has at most $|P| + |T|$ nodes and $|F|$ arcs.
  Each of the two passes visits every node and every arc once, and the computation of $Sigma$, the level assignment and the final `flat_map` visit every node once.
]

#theorem(title: "Correctness")[
  For every transition $t$, the algorithm returns $ell[K(t)]$ with: $lv(0)$ iff $t$ is dead, $lv(1)$ iff $t$ is strictly $lv(1)$-live, $lv(3)$ iff $t$ is strictly $lv(3)$-live, and $lv(4)$ iff $t$ is live.
  _Proof._ The four cases of the Algorithm lemma are exhaustive and mutually exclusive.
  The case $lv(0)$ is part (1) of the Classification lemma: since $s_t$ is the only predecessor of $t$, a marked place reaches $s_t$ iff it reaches $t$, i.e. iff $K(t)$ is reachable from $P_0$.
  The case $lv(1)$ is part (2), $lv(4)$ is parts (4) combined with the Live SCCs lemma, and for $lv(3)$ we use part (3) and the fact that $t$ is not live by part (4).
]

#corollary[
  No transition of a state machine is strictly $lv(2)$-live.
  For a strongly connected state machine with $M_0(P) > 0$, the algorithm returns $lv(4)$ for every transition, which agrees with @murata_petri_1989[Thm. 4] and @esparza_petri_2026[Thm. 5.1.3].
  _Proof._ A fed transition is either $lv(1)$ and not $lv(2)$, or $lv(3)$ and hence $lv(2)$.
  A strongly connected net has one SCC, which is terminal and contains the only terminal SCC reachable from any marked place.
]