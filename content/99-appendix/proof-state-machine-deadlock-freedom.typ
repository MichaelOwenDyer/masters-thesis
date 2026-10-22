#import "../lib.typ": *
#import "../../lib/tum/elements.typ": *

== Proof of Correctness of @algorithm-state-machine-deadlock-freedom <proof-state-machine-deadlock-freedom>

The proof uses the following known facts.

+ _Token movement_ @esparza_petri_2026[Thm. 5.1.3]: if $q$ is marked and $q reach r$, the transitions on the path can be fired in order, which moves one token from $q$ to $r$ and leaves the other tokens in place.
+ _Enabledness in state machines_: a transition $t$ is enabled at $M$ iff $M(s_t) > 0$, where ${s_t} := pre(t)$ (the unique input place of $t$).

Let $||M_0|| = {p in P | M_0(p) > 0}$ be the support of the initial marking $M_0$.

Let $Sigma = {p in P | post(p) = emptyset}$ be the set of sink places.

#lemma(title: "Deadlock markings")[

  A marking $M$ is a deadlock iff every marked place of $M$ has an empty postset.

  _Proof._ By Fact 2, $M$ enables no transition iff $M(s_t) = 0$ for every $t in T$.
  The places of the form $s_t$ are exactly the places with a nonempty postset, so this holds iff all marked places have an empty postset.
]

#lemma(title: "Reachable deadlocks")[

  A deadlock is reachable from $M_0$ iff $||M_0|| subset.eq prestar(Sigma)$.

  _Proof._
  ($arrow.l.double$) For every $q in ||M_0||$ pick $r_q in poststar(q) inter Sigma$.
  By Fact 1 we move every token of every marked place $q$ to $r_q$.
  The resulting marking has tokens only on sink places and is a deadlock by the previous lemma.
  If $||M_0|| = emptyset$, then $M_0$ itself is a deadlock.
  ($arrow.r.double$) Let $M$ be a reachable deadlock and let $q in ||M_0||$.
  The set $poststar(q)$ is closed under successors, so a firing either does not touch $poststar(q)$, moves a token into it, or moves a token within it.
  Hence the number of tokens on $poststar(q)$ never decreases, and $M(poststar(q)) >= M_0(q) >= 1$.
  So some place $r in poststar(q)$ is marked at $M$, and by the previous lemma $r$ is a sink place.
  Thus $q$ reaches a sink place, i.e. $q in prestar(Sigma)$.
]

#lemma(title: "Backward search")[
  $"PreStar"(X)$ returns $prestar(X)$ and runs in time $O(|P| + |T| + |F|)$.

  _Proof._

  _Soundness._ Every place added to $X$ is a predecessor $q$ of some transition $t in pre(p)$, where $p$ was already in the set.
  So $q reach t reach p$, and inductively $q$ reaches a place of the initial $X$.

  _Completeness._ Let $q reach r$ with $r in X$, via a path $q = p_0, t_1, p_1, dots, t_k, p_k = r$.
  Every place in $X$ is pushed onto $W$ exactly once, either initially or when it is added, so every place of the final set is popped.
  By induction from the end of the path, $p_i$ is in the set: when $p_(i+1)$ is popped, $t_(i+1) in pre(p_(i+1))$ and $p_i in pre(t_(i+1))$ are scanned, and $p_i$ is added unless it is already present.

  _Complexity._ Each place is pushed and popped at most once.
  Popping $p$ scans $pre(p)$ and, for each $t in pre(p)$, the single place $s_t$.
  Since every transition has exactly one output place, each transition $t$ is scanned exactly once over the whole run, so the total work is $O(|P| + |T| + |F|)$.
]

#theorem(title: "Correctness")[
  The algorithm returns true iff $(N, M_0)$ is deadlock-free.
  It runs in time $O(|P| + |T| + |F|)$.

  _Proof._ By the Reachable deadlocks lemma, $(N, M_0)$ is deadlock-free iff $||M_0|| subset.eq.not prestar(Sigma)$, i.e. iff some marked place cannot reach a sink place.
  By the Backward search lemma, $B = prestar(Sigma)$.
  The algorithm returns exactly $||M_0|| subset.eq.not B$.
  The computation of $Sigma$ and the subset test are linear in $|P|$.
]

#corollary[
  A state machine whose initial marking has no token is never deadlock-free.
  A state machine without sink places is deadlock-free iff $M_0(P) > 0$.
  _Proof._ If $||M_0|| = emptyset$, $M_0$ is a deadlock.
  If $Sigma = emptyset$, then $prestar(Sigma) = emptyset$, and $||M_0|| subset.eq.not emptyset$ iff $||M_0|| != emptyset$.
]