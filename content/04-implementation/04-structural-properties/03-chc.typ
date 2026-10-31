#import "../../lib.typ": *
#import "@preview/lovelace:0.3.0": *

#let fold = $op("fold")$
#let map  = $op("map")$
#let zip  = $op("zip")$

=== Commoner/Hack Criterion <impl-chc>

The Commoner/Hack criterion (CHC), also called the Siphon-Trap Criterion, states that every proper siphon contains a marked trap.
We encode the search for a CHC counterexample as a SAT problem following the approach of @oanea_new_2010 et al.

The CHC fails iff there exists a siphon such that all traps contained in it are unmarked.
Because this is an $exists forall$ statement, we cannot encode it directly.
Instead we exploit that every set of places $D$ contains a unique maximal trap (the union of all traps in $D$),
and compute it within the SAT encoding.
We then assert that this maximal trap is unmarked, which is equivalent to asserting that all traps in $D$ are unmarked.

*Siphons.*
We introduce one Boolean variable $x_p^((0))$ per place $p in P$, with the intended meaning
$x_p^((0)) = top$ iff $p in D$.

The assignments satisfying

$ phi_"siphon" := or.big_(p in P) x_p^((0)) and and.big_(t in T) and.big_(p in post(t)) (x_p^((0)) -> or.big_(q in pre(t)) x_q^((0))) $

correspond exactly to the siphons of $N$ @oanea_new_2010[Lemma 1].
The first conjunct enforces $D != emptyset$ (at least one place is in the siphon).
The second states $pre(D) subset.eq post(D)$: if a transition $t$ produces into $D$, it must also consume from $D$.
In the implementation the inner conjunction over $p in post(t)$ is folded into the premise
$or.big_(p in post(t)) x_p^((0)) -> or.big_(q in pre(t)) x_q^((0))$, which is logically equivalent.

*Maximal trap.*
Given a siphon $D$, define the sequence
$ D_0 := D, quad D_(i+1) := { p in D_i | forall t in post(p): thin post(t) inter D_i != emptyset }. $
That is, $D_(i+1)$ removes every place that has a consuming transition producing nothing into $D_i$.
This is the unrolling of the iterative algorithm for computing the maximal trap contained in $D$
as described (in its siphon dual form) in @best_petri_2024[Alg. 6.19].

#emph[Lemma.] For every $D subset.eq P$, $D_(|P|)$ is the maximal trap contained in $D$ if it is nonempty, and no trap is contained in $D$ if $D_(|P|) = emptyset$.

#emph[Proof.]
The sequence is non-increasing, so it either strictly shrinks or stays the same at each step.
Since $|D_0| <= |P|$, it reaches a fixpoint at the latest at $D_(|P|)$, i.e. $D_(|P|+1) = D_(|P|)$.
A fixpoint $D_(|P|)$ satisfies $post(D_(|P|)) subset.eq pre(D_(|P|))$ by construction, so it is a trap when nonempty.
Conversely, let $Q subset.eq D$ be any trap. We show $Q subset.eq D_i$ by induction on $i$.
The base case is $Q subset.eq D_0$.
For the step, let $p in Q$ and $t in post(p)$. Since $Q$ is a trap, $t in pre(Q)$, so $post(t) inter Q != emptyset$.
By the induction hypothesis $post(t) inter D_i != emptyset$, hence $p in D_(i+1)$.
Therefore $Q subset.eq D_(|P|)$, which makes $D_(|P|)$ maximal. $qed$

We unroll this iteration inside the formula.
For each stage $i in {1, dots, |P|}$ and place $p in P$ we define the term
$ x_p^((i)) := x_p^((i-1)) and and.big_(t in post(p)) or.big_(q in post(t)) x_q^((i-1)). $
The term $x_p^((i))$ is true iff $p in D_i$.
This unrolling uses only the original $|P|$ free variables and no auxiliary variables.
Stage $i$ adds one term per place, and each term has size $O(|post(p)| dot max_t |post(t)|)$, so the total size is $O(|P| dot (|P| + |T| + |F|))$.

*Unmarked trap.*
The maximal trap must be unmarked or empty:
$ phi_"unmarked" := and.big_(p in P : M_0(p) > 0) not x_p^((|P|)). $

*Correctness.*
#emph[Theorem.]
The formula $phi := phi_"siphon" and phi_"unmarked"$ is satisfiable iff $N$ has a proper siphon that contains no marked trap.
Equivalently, $phi$ is unsatisfiable iff the CHC holds.

#emph[Proof.]
($arrow.r.double$) Let $beta$ satisfy $phi$ and set $D := {p | beta(x_p^((0))) = top}$.
By $phi_"siphon"$, $D$ is a proper siphon.
By the Lemma, $D_(|P|)$ is the maximal trap in $D$, and $phi_"unmarked"$ gives $M_0(D_(|P|)) = 0$.
Any trap $Q subset.eq D$ satisfies $Q subset.eq D_(|P|)$, so $M_0(Q) = 0$. \
($arrow.l.double$) Let $D$ be a proper siphon with no marked trap, and let $beta$ assign $x_p^((0))$ true iff $p in D$.
Then $phi_"siphon"$ holds.
The maximal trap $D_(|P|)$ of $D$ is unmarked or empty, so $phi_"unmarked"$ holds. $qed$

When $phi$ is satisfiable, the assignment gives a counterexample: the siphon $D$ and its maximal trap $D_(|P|)$ (possibly empty).

The following pseudocode mirrors the implementation of the above encoding in `petrivet`,
which returns a counterexample if one exists, or $bot$ if the CHC holds.

#figure(
  kind: "algorithm",
  supplement: "Algorithm",
  caption: [Counterexample search for the CHC as a Boolean formula.],
  pseudocode-list(numbered-title: [`FindProperSiphonWithUnmarkedTrap`$(|P|, M_0)$])[
    + *for each* $p in P$: introduce a Boolean variable $x_p$
    + $bold(x)^((0)) <- (x_p)_(p in P)$ #h(1em) // `in_siphon`
    + *assert* $or.big_(p in P) x^((0))_p$ #h(1em) // proper siphon
    + *for each* $t in T$:
      + *assert* $(or.big_(p in post(t)) x^((0))_p) -> (or.big_(p in pre(t)) x^((0))_p)$
    + *define* $"Step"(bold(x)) := zip(P, bold(x)) pipe map((p, x_p) |-> x_p and and.big_(t in post(p)) or.big_(q in post(t)) x_q)$
    + $bold(x)^((|P|)) <- fold(bold(x)^((0)), thin |P| "times", "Step")$ #h(1em) // `in_trap`
    + *for each* $p in P$ *with* $M_0(p) > 0$:
      + *assert* $not x^((|P|))_p$
    + *if* the solver reports UNSAT: *return* $bot$
    + $D <- {p in P | beta(x^((0))_p) = top}$
    + $Q <- {p in P | beta(x^((|P|))_p) = top}$
    + *return* $(D, Q)$
  ]
)
