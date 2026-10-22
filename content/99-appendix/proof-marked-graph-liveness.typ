#import "../lib.typ": *
#import "../../lib/tum/elements.typ": *

== Proof of Correctness of @algorithm-marked-graph-liveness <proof-marked-graph-liveness>

The proof reduces to four known facts about marked graphs (T-systems).

+ _Token invariance_ @murata_petri_1989[Thm. 6] @esparza_petri_2026[Prop. 5.2.2]:
  $M(gamma) = M_0(gamma)$ for every circuit $gamma$ and every reachable marking $M$.
  Hence an unmarked circuit stays unmarked, so $Z := {t in T | exists gamma : v in gamma and M_0(gamma) = 0}$
  does not depend on the marking, and no transition of $Z$ is ever enabled (this is the (⇒) direction of @esparza_petri_2026[Thm. 5.2.3]).
+ _Necessity of firing_ (@murata_petri_1989[§VII-D]): if $N$ has a token-free directed path from $i$ to $j$ at $M_0$, then $i$ must fire before $j$ can first become enabled.
+ _Local liveness_ (the (⇐) direction of @esparza_petri_2026[Thm. 5.2.3]):
  Let $M$ be a marking and $t$ a transition such that no circuit unmarked at $M$ has a token-free path to $t$.
  Then some marking reachable from $M$ enables $t$.
  The proof of the cited theorem uses its hypothesis "every circuit is marked at $M$" only to guarantee that a maximal token-free path $Pi$ ending in $t$ has a transition as first element whose input places are all marked.
  If a place were unmarked, extending $Pi$ would either yield a longer token-free path or close a circuit inside $Pi$ that is unmarked and has a token-free path to $t$, which our hypothesis excludes.
  The induction measure $|S_M|$ and its decrease under firing $M arrow.r^u L$ ($S_L subset S_M$) do not use the hypothesis again, and the hypothesis is preserved by firing by token invariance.
+ _Graph algorithms_: the strongly connected components of a digraph are computed in linear time @tarjan_depth-first_1972, and a node lies on a directed cycle iff its component has at least two nodes.

#lemma(title: "Unmarked circuits")[
  Let $N_0$ be the subgraph of $N$ induced by the unmarked places at $M_0$ and all transitions.
  $Z$ is the set of transitions in nontrivial strongly connected components of $N_0$.

  _Proof._ $N_0$ contains exactly the unmarked places, so a directed cycle of $N_0$ is an unmarked circuit and vice versa. The claim is Fact 4.
]

#lemma(title: "Classification")[

  Let $t in T$.

  Let $D_0 := {t in T | exists z in Z: z reachz t}$ be the set of transitions reachable from an unmarked circuit via a token-free path at $M_0$.

  Let $D_(>=) := {t in T | exists z in Z: z reach t}$ be the set of transitions reachable from an unmarked circuit via any path at $M_0$.

  + If $t in D_0$, then $t$ is dead.
  + If $t in.not D_0$, then $t$ is potentially firable ($lv(1)$-live).
  + If $t in D_(>=)$, then $t$ fires boundedly often in every firing sequence, hence is not $lv(2)$-live.
  + If $t in.not D_(>=)$, then $t$ is $lv(4)$-live.

  _Proof._

  (1) By Fact 1 every $z in Z$ is dead.
  If $z reachz t$, Fact 2 shows that $z$ would have to fire before $t$ can become enabled, so $t$ is dead as well.

  (2) Apply Fact 3 to $M = M_0$: the hypothesis is exactly $t in.not D_0$.

  (3) Let $z = t_0, p_1, t_1, dots, p_k, t_k = t$ be a path with $z in Z$ and let $sigma$ be a firing sequence.
  Since $t_(i-1)$ and $t_i$ are the only producer and consumer of $p_i$, the marking equation gives $0 <= M(p_i) = M_0(p_i) + arrow(sigma) (t_(i-1)) - arrow(sigma) (t_i)$.
  As $z$ is dead by (1), $arrow(sigma) (t_0) = 0$, and induction yields $arrow(sigma) (t) <= sum_i M_0(p_i)$.

  (4) Let $M$ be reachable. If a circuit were unmarked at $M$ and had a path to $t$, it would be unmarked at $M_0$ by Fact 1, so $t in D_(>=)$.
  Hence the hypothesis of Fact 3 holds at every reachable $M$, so $t$ can always be enabled again.
]

#lemma(title: "Depth-first search")[
  After the algorithm terminates, $ell(t) = lv(0)$ if $t in D_0$, $lv(1)$ if $t in D_(>=) without D_0$, and $lv(4)$ otherwise.
  It runs in time $O(|P| + |T| + |F|)$.

  _Proof._

  A call $"Dfs"(t, b)$ passes the `match` without returning only if $ell[t] = lv(4)$, or $ell[t] = lv(1)$ and $b = bot$; in both cases the assignment strictly lowers $ell[t]$.
  So every transition is expanded at most twice, which bounds the work by $O(|F|)$, and together with Fact 4 the total is linear.
  The search explores the product graph on states $(t, b)$ with $b in {bot, top}$, where passing a marked place sets $b$ to $top$.
  Arriving at $t$ through unmarked places only ($bot$) is at least as strong as arriving through a path that crossed a marked place ($top$), because the same continuation from $(t, bot)$ reaches every transition that $(t, top)$ reaches, with a flag that is never high.
  The pruning in the `match` discards exactly the states that were already visited with a high flag.
  Therefore the algorithm computes reachability in the product graph from the states ${(z, bot) | z in Z}$ @tarjan_depth-first_1972, and $ell(t)$ is $lv(0)$ iff some $(t, bot)$ is reachable (i.e. $t in D_0$), $lv(1)$ iff only $(t, top)$ is reachable, and $lv(4)$ iff no state of $t$ is reachable.
]

#theorem(title: "Correctness")[
  The algorithm returns $ell$ with $ell(t) = lv(0)$ iff $t$ is dead, $ell(t) = lv(1)$ iff $t$ is strictly $lv(1)$-live, and $ell(t) = lv(4)$ iff $t$ is live.

  _Proof._ By the Depth-first search lemma, $ell$ is determined by the sets $D_0 subset.eq D_(>=)$.
  By the Classification lemma, the transitions in $D_0$ are dead, those in $D_(>=) without D_0$ are $lv(1)$-live but not $lv(2)$-live, and those outside $D_(>=)$ are live.
  These three cases are exhaustive and mutually exclusive, which gives the equivalences.
]

#corollary[
  In a marked graph, no transition is strictly $lv(2)$-live or strictly $lv(3)$-live.
]