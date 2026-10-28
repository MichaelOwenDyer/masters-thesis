#import "../../lib/tum/elements.typ": *

== (K-)Boundedness

This section describes the (k-)boundedness property of Petri nets and specific results for subclasses of Petri nets.

*Strongly connected state machine.*

*All places are k-bounded iff $M_0(S) ≤ k$* @esparza_petri_2026[Prop. 5.1.2]

The state machine property ensures that every transition moves exactly one token,
so the total number of tokens in the system is invariant (conservativeness).
The strongly connected property ensures that every token can reach every place,
so every place is bounded by the total number of tokens in the system.

[$O(|P|)$],
,
[Implemented],

[State machine],
[A place p is k-bounded iff M₀(pre*(p)) ≤ k, where pre*(p) is the set of places which can reach p via a directed path of transitions and places.],
[
  Take pre* to be reflexive.
  No transition can bring a token into pre*(p) from outside, because t outputs into pre*(p) implies •t ∈ pre*(p).
  So M(pre*(p)) never increases.
  Every token in pre*(p) can be walked to p, one token at a time.
],


[Live Marked Graph],
[A place $s$ is bounded iff it lies on a circuit $γ$, and k-bounded iff $min{M_0(γ) : s ∈ γ} ≤ k$.],
[
  A place is exactly bounded by the minimum token count of the circuits it lies on,
  and unbounded if it does not lie on any circuit.
],
[Polynomial],
[@esparza_petri_2026[Cor. 5.2.5 (1, 2)]; @best_petri_2024[Prop. 5.40]; @murata_petri_1989[Thm. 8]],
[Implemented],

[Non-live Marked Graph],
[
  A place $s$ on a circuit $γ$ is k-bounded if $M_0(γ) ≤ k$, but may additionally be bounded for lower k.
],
[
  At least one circuit is unmarked.
  Unmarked circuits can prevent tokens on other circuits from circulating,
  leading to a lower bound for some places than otherwise.
],
[Polynomial],
[], // TODO
[Implemented],

[Live Free-Choice Net],
[
  A place is bounded iff it resides in an S-component (subnet strongly connected state machine).
  A place $s$ is k-bounded iff some strongly connected S-component $S_1 ∋ s$ has $M_0(S_1) ≤ k$.
],
[
  The token counts of state machines are invariant, and the same applies to state machines embedded in a free-choice net.
],
[NP-complete (liveness precondition)],
[Hack's boundedness theorem: @esparza_petri_2026[Thm. 5.3.14, Prop. 5.3.15]; @best_petri_2024[Thm. 5.34, Cor. 5.36]; @desel_free_1995; @hack_analysis_1972],
[Not implemented],

[General],
[
  $y ≫ 0$ with $C^T y ≤ 0$ ⇒ $N$ is bounded under every $M_0$ ("structurally bounded").
  A place $s$ is bounded by $⌊(y · M_0) / y(s)⌋$.
],
[
  Semi-algorithm: a positive place subinvariant is a weighted sum of the token counts of all places which is non-increasing under every transition firing.
  If such a weight vector exists, then the net is bounded under every initial marking,
  and (not necessarily minimal) place bounds can be computed from the weight vector and the initial marking.
],
[P],
[@best_petri_2024[Prop. 4.12]; @murata_petri_1989[Thm. 29]],
[Implemented],

[General],
[The marking-equation LP $max ∑_s∈S M(s)$ subject to $M = M_0 + C X$, $X ≥ 0$ has a finite optimum ⇒ the Petri net is bounded.],
[
  Semi-algorithm: an algebraic finite maximum of the total token count within the marking equation implies that the Petri net is bounded.
  The converse is not true, as the marking equation is only an overapproximation of the reachable markings.
],
[P],
[@esparza_petri_2026[Prop. 4.2.7]],
[Not implemented],

[General],
[Karp–Miller coverability graph: $s$ is unbounded iff some node carries $ω$ at $s$; $s$ is $k$-bounded iff every node has $M(s) ≤ k$],
[
  Constructing the coverability graph is a complete procedure for checking (k-)boundedness, but it is expensive.
  Use as a last resort when no other procedure yields a definitive answer.
],
[EXPSPACE-complete (@rackoff_covering_1978 @lipton_reachability_1976); the graph can be non-primitive-recursive in size @wimmel_applying_2011],
[@karp_parallel_1969; @esparza_petri_2026[Thm. 3.2.4, 3.2.5]; @best_petri_2024[Prop. 3.23, 3.27]],
[Implemented]

Scope:
- Check an individual place.
- Check the entire net.

Operation:
- Decide whether k-bounded for any `k`.
- Given `k`, decide whether k-bounded.
- Find the minimal `k` such that the place (or Petri net) is k-bounded.

Together with the efficiency restriction dimension, this yields 2 x 3 x 2 = 12 different variants
for checking (k-)boundedness, which `petrivet` provides as methods on `PetriNet`:

*`pn.is_bounded() -> bool`*

Check if the Petri net is bounded.

*`pn.is_place_bounded(p) -> bool`*

Check if the Place `p` is bounded.

*`pn.is_efficiently_bounded() -> Option<bool>`*

Check if the Petri net is bounded using only efficient procedures, returning `None` otherwise.

*`pn.is_place_efficiently_bounded(p) -> Option<bool>`*

Check if the Place `p` is bounded using only efficient procedures, returning `None` otherwise.

*`pn.is_k_bounded(k) -> bool`*

Check if the Petri net is k-bounded.

*`pn.is_place_k_bounded(p, k) -> bool`*

Check if the Place `p` is k-bounded.

*`pn.is_efficiently_k_bounded(k) -> Option<bool>`*

Check if the Petri net is k-bounded using only efficient procedures, returning `None` otherwise.

*`pn.is_place_efficiently_k_bounded(p, k) -> Option<bool>`*

Check if the Place `p` is k-bounded using only efficient procedures, returning `None` otherwise.

*`pn.boundedness() -> Boundedness`*

Find the minimal k-boundedness of the Petri net.

*`pn.place_boundedness(p) -> Boundedness`*

Find the minimal k-boundedness of a particular place `p`.

*`pn.efficient_boundedness() -> Option<Boundedness`*

Find the minimal k-boundedness of the Petri net using only efficient procedures, returning `None` otherwise.

*`pn.efficient_place_boundedness(p) -> Option<Boundedness>`*

Find the minimal k-boundedness of a particular place `p` using only efficient procedures, returning `None` otherwise.

All of these methods are implemented with a single call to a private method which takes the property scope, desired result granularity, and efficiency restriction as parameters,
and dispatches to the appropriate procedure based on the structural subclass of the net and the desired result granularity.

Results are internally cached on the PetriNet structure to reuse in subsequent calls.

We represent this property in Rust as an enum:
```rust
pub enum Boundedness {
  Bounded(K),
  Unbounded,
}
```
where `K` is a type alias for `u32`, representing the smallest `k` for which the place (or Petri net) is k-bounded.
