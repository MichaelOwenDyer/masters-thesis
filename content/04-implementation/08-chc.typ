#import "../lib.typ": *
#import "@preview/lovelace:0.3.0": *

#let fold = $op("fold")$
#let map  = $op("map")$
#let zip  = $op("zip")$

== Commoner-Hack Criterion <chc>

The Commoner-Hack criterion (CHC), also called the Siphon-Trap Criterion, states that every siphon contains a marked trap.

=== SAT Encoding of the CHC Counterexample Search

We encode the search for a CHC counterexample as a SAT problem following the approach of @oanea_new_2010 et al.

The CHC fails iff there exists a siphon such that all traps contained in it are unmarked.
Because this is an $exists forall$ statement, we cannot encode it directly.
Instead we exploit that every set of places $D$ contains a unique maximal trap (the union of all traps in $D$),
and compute it within the SAT encoding.
We then assert that this maximal trap is unmarked, which is equivalent to asserting that all traps in $D$ are unmarked.

*Step 1: Siphons.*
We introduce one Boolean variable $x_p^((0))$ per place $p in P$, with the intended meaning
$x_p^((0)) = top$ iff $p in D$.

The assignments satisfying

$ phi_"siphon" := or.big_(p in P) x_p^((0)) and and.big_(t in T) and.big_(p in post(t)) (x_p^((0)) -> or.big_(q in pre(t)) x_q^((0))) $

correspond exactly to the siphons of $N$ @oanea_new_2010[Lemma 1].
The first conjunct enforces $D != emptyset$ (at least one place is in the siphon).
The second states $pre(D) subset.eq post(D)$: if a transition $t$ produces into $D$, it must also consume from $D$.
In the implementation the inner conjunction over $p in post(t)$ is folded into the premise
$or.big_(p in post(t)) x_p^((0)) -> or.big_(q in pre(t)) x_q^((0))$, which is logically equivalent.

*Step 2: Maximal trap.*
Given a siphon $D$, define the sequence
$ D_0 := D, quad D_(i+1) := { p in D_i | forall t in post(p): thin post(t) inter D_i != emptyset }. $
That is, $D_(i+1)$ removes every place that has a consuming transition producing nothing into $D_i$.
This is the unrolling of the iterative algorithm for computing the maximal trap contained in $D$
as described (in its siphon dual form) in @best_petri_2024[Alg. 6.19].

#emph[Lemma.] For every $D subset.eq P$, $D_n$ is the maximal trap contained in $D$ if it is nonempty, and no trap is contained in $D$ if $D_n = emptyset$.

#emph[Proof.]
The sequence is non-increasing, so it either strictly shrinks or is stationary.
Let $n := |P|$ be the total number of places in the net.
Since $|D_0| <= n$, it reaches a fixpoint at the latest at $D_n$, i.e. $D_(n+1) = D_n$.
A stationary $D_n$ satisfies $post(D_n) subset.eq pre(D_n)$ by construction, so it is a trap when nonempty.
Conversely, let $Q subset.eq D$ be any trap. We show $Q subset.eq D_i$ by induction on $i$.
The base case is $Q subset.eq D_0$.
For the step, let $p in Q$ and $t in post(p)$. Since $Q$ is a trap, $t in pre(Q)$, so $post(t) inter Q != emptyset$.
By the induction hypothesis $post(t) inter D_i != emptyset$, hence $p in D_(i+1)$.
Therefore $Q subset.eq D_n$, which makes $D_n$ maximal. $qed$

We unroll this iteration inside the formula.
For each stage $i in {1, dots, n}$ and place $p in P$ we define the term
$ x_p^((i)) := x_p^((i-1)) and and.big_(t in post(p)) or.big_(q in post(t)) x_q^((i-1)). $
The term $x_p^((i))$ is true iff $p in D_i$.
These terms are not fresh variables; they are macros over $x^((0))$, so the formula has only $n$ free variables.
The encoding is polynomial in size.
Stage $i$ adds one term per place, and each term has size $O(|post(p)| dot max_t |post(t)|)$, so the total size is $O(n dot (|P| + |T| + |F|))$.

*Step 3: Unmarked trap.*
The maximal trap must be unmarked or empty:
$ phi_"unmarked" := and.big_(p in P : M_0(p) > 0) not x_p^((n)). $

*Step 4: Correctness.*
#emph[Theorem.]
The formula $phi := phi_"siphon" and phi_"unmarked"$ is satisfiable iff $N$ has a siphon that contains no marked trap.
Equivalently, $phi$ is unsatisfiable iff the CHC holds.

#emph[Proof.]
($arrow.r.double$) Let $beta$ satisfy $phi$ and set $D := {p | beta(x_p^((0))) = top}$.
By $phi_"siphon"$, $D$ is a siphon.
By the Lemma, $D_n$ is the maximal trap in $D$, and $phi_"unmarked"$ gives $M_0(D_n) = 0$.
Any trap $Q subset.eq D$ satisfies $Q subset.eq D_n$, so $M_0(Q) = 0$.
($arrow.l.double$) Let $D$ be a siphon with no marked trap, and let $beta$ assign $x_p^((0))$ true iff $p in D$.
Then $phi_"siphon"$ holds.
The maximal trap $D_n$ of $D$ is unmarked, or empty, so $phi_"unmarked"$ holds. $qed$

When $phi$ is satisfiable, the assignment gives a counterexample: the siphon $D$ and its maximal trap $D_n$ (possibly empty).

The following pseudocode mirrors the implementation of the above encoding in petrivet,
which returns a counterexample if one exists, or $bot$ if the CHC holds.

#figure(
  kind: "algorithm",
  supplement: "Algorithm",
  caption: [Counterexample search for the CHC as a Boolean formula.],
  pseudocode-list(numbered-title: [`FindSiphonWithUnmarkedMaximalTrap`$(N, M_0)$])[
    + $n <- |P|$
    + *for each* $p in P$: introduce a fresh Boolean variable $x_p$
    + $bold(x)^((0)) <- (x_p)_(p in P)$ #h(1em) // `in_siphon`
    + *assert* $or.big_(p in P) x^((0))_p$ #h(1em) // proper siphon
    + *for each* $t in T$:
      + *assert* $(or.big_(p in post(t)) x^((0))_p) -> (or.big_(p in pre(t)) x^((0))_p)$
    + *define* $"Step"(bold(x)) := zip(P, bold(x)) pipe map((p, x_p) |-> x_p and and.big_(t in post(p)) or.big_(q in post(t)) x_q)$
    + $bold(x)^((n)) <- fold(bold(x)^((0)), thin n "times", "Step")$ #h(1em) // `in_trap`
    + *for each* $p in P$ *with* $M_0(p) > 0$:
      + *assert* $not x^((n))_p$
    + *if* the solver reports UNSAT: *return* $bot$
    + $D <- {p in P | beta(x^((0))_p) = top}$
    + $Q <- {p in P | beta(x^((n))_p) = top}$
    + *return* $(D, Q)$
  ]
)
