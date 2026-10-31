#import "../../lib.typ": *

=== P-Invariants <impl-p-invariants>

P-invariants, P-subinvariants, and P-surinvariants (see @background-invariants) are used by several procedures in `petrivet`,
each of which needs an invariant with some additional property.
All of them share the same basic encoding as an SMT problem over integer variables,
which consists of two parts: what *kind* of invariant is being sought, and *where* to look for it.

*Kind of invariant.*
We introduce one integer variable $y_p >= 0$ per place $p in P$ to represent its weight in the invariant.
For each transition $t in T$, the weighted effect of firing $t$ is $sum_(p in P) C(p, t) dot y_p$,
the change in the weighted token sum when $t$ fires.
Depending on the kind of invariant, we constrain this effect with a relation ${=, <=, >=}$ for every transition:

$ phi_"kind" := and.big_(t in T) sum_(p in P) C(p, t) dot y_p med bowtie med 0 \
  quad "where" bowtie = cases(
    = & "for a P-invariant (constant weighted sum)",
    <= & "for a P-subinvariant (non-increasing)",
    >= & "for a P-surinvariant (non-decreasing)",
  )
$

In the implementation, transitions which no effect on any place (self-loops only) give the trivial constraint $0 bowtie 0$
and are omitted from the SMT problem.

Every solution $y$ of $phi_"kind"$ is an invariant of the requested kind, but so is the trivial solution $y = 0$,
which is not useful.
On its own, $phi_"kind"$ is therefore not useful: we must add constraints which say where the invariant should be.

*Covering a set of places.*
For a set of places $S subset.eq P$ that the invariant should cover, we require each of them to have positive weight in $y$:
$ phi_"cover"(S) := and.big_(p in S) y_p >= 1 $
For $S = P$ this asks for a positive invariant covering the whole net.

*Separating two markings.*
It is also useful to seek an invariant which proves that a marking is not reachable from another marking,
because they disagree on the constant or monotone weighted token sum of the invariant.
For two markings $M_A$ and $M_B$, we require the weighted token sums $y dot M = sum_(p in P) y_p dot M(p)$
to _violate_ the invariant when going from $M_A$ to $M_B$:

$
phi_"separate"(M_A, M_B) := cases(
  y dot M_B != y dot M_A & "for a P-invariant",
  y dot M_B > y dot M_A & "for a P-subinvariant",
  y dot M_B < y dot M_A & "for a P-surinvariant",
)
$

This constraint excludes the trivial solution on its own, since $y = 0$ gives $y dot M_A = y dot M_B = 0$.
A solution of $phi_"kind" and phi_"separate"(M_A, M_B)$ proves that $M_B$ is not reachable from $M_A$:
by the marking equation, every $M$ reachable from $M_A$ satisfies $M = M_A + C x$ for some $x in NN^(|T|)$, so
$ y dot M = y dot M_A + (y^T C) x. $
For a P-invariant $y^T C = 0$, so $y dot M = y dot M_A$.
For a P-subinvariant $y^T C <= 0$ and $x >= 0$, so $y dot M <= y dot M_A$.
For a P-surinvariant, symmetrically, $y dot M >= y dot M_A$. \
In each case, $M_B$ is excluded.

If the solver reports a model, then the assignment of the $y_p$ is the invariant.
Its support $supp(y) = {p in P : y_p > 0}$ gives the places it assigns positive weight to,
and all other places are irrelevant to the invariant.

The covering constraint is used in @impl-boundedness to decide structural boundedness,
and the separating constraint is used in @impl-reachability-coverability
to refine the CEGAR abstraction.