
=== Liveness

A transition is said to be _live_ if, from every reachable marking, it is possible to eventually fire it again;
that is, there is no reachable marking from which the transition can never be fired again.
A Petri net is said to be live if all of its transitions are live.

We can further specify liveness by classifying it into four levels, L0 through L4 @murata_petri_1989, which I explain here in my own words:
- L0 ("dead"): No firing sequence contains the transition. The transition is not enabled in any reachable marking.
- L1 ("not dead"): There exists a firing sequence containing the transition. The transition is enabled in at least one reachable marking.
- L2 (implies L1): For any finite integer k, there exists a firing sequence containing the transition k times.
- L3 (implies L2): There exists an infinite firing sequence containing the transition infinitely many times. The transition can be fired infinitely often, but there may be reachable markings where this is no longer possible.
- L4 ("live", implies L3): From every reachable marking, there exists a firing sequence containing the transition. The transition can always be fired again.

#figure(
  kind: table,
  caption: [
    Known results for (L4-)liveness and the liveness levels, and their implementation status.
    Direction: $⇔$ the condition characterises the property, $⇒$ the condition is sufficient, $⇐$ the condition is necessary.
  ],
  table(
    columns: (auto, auto, auto, auto, auto, auto),
    [*Applicability*], [*Theorem*], [*Intuition*], [*Complexity*], [*Citation*], [*Implementation status*],

    [Strongly Connected State Machine],
    [All transitions are L4 live iff the initial marking contains at least one token],
    [Tokens are conserved and can reach every transition arbitrarily often.],
    [O(|P|)],
    [@esparza_petri_2026[Thm. 5.1.3]; @murata_petri_1989[Thm. 4, Thm. 7]],
    [Implemented],

    [State Machine],
    [
      Transitions between SCCs are L1;
      transitions inside a nontrivial SCC downstream of a token are at least L3;
      L4 if that SCC is the only terminal nontrivial SCC reachable from some marked SCC;
      L0 if the SCC is unmarked and not downstream of any marked SCC.
    ],
    [
      TODO: Actual proof
      L0 means unreachable from every marked place.
      L1 means reachable, but not on a cycle, so the transition fires at most `M₀(pre*(•t))` times.
      L3 means reachable and on a cycle. L2 and L3 coincide in an S-net, so your code never emits L2. Say that in the thesis.
      L4 means some marked q has `post*(q) ⊆ pre*(•t)`. That forces the SCC to be terminal, which is what your "unique reachable terminal SCC" test checks. State this lemma in the proof, because it's the real reason for the single-terminal rule.
    ],
    [],
    [],
    [],

    [Marked Graph],
    [],
  ),
) <liveness-theorems>

We can represent this property in Rust as an enum:

```rust
pub enum LivenessLevel {
  L0,
  L1,
  L2,
  L3,
  L4,
}
```

These granular liveness levels are at least of theoretical interest, but as expected by the terminology, when unspecified, liveness is assumed to mean L4.

The liveness level of a Petri net is defined as the minimum liveness level of its transitions.
I find this to be a rather unhelpful definition, as the minimum liveness level is rarely representative of the net as a whole.
Consider a Petri net of 100 transitions, where 99 of them are a nuanced mixture of L1 through L4, and one of them is dead (L0).
The net is considered dead (L0) overall, but this does not provide any useful information about the other 99 transitions.
Furthermore, from a computational perspective, determining the minimum liveness level of a net is no easier in general than determining
the liveness level of each individual transition, so we might as well provide the user with the more useful information.

Therefore, we can classify the problem of checking liveness along three dimensions:

Operation:
- Check whether L4 or not L4 -> bool.
- Determine the precise liveness level -> LivenessLevel.

Scope:
- Check the aggregate liveness of the entire net.
- Check an individual transition.
- Check all transitions.

These scopes are not mutually exclusive; sometimes their results are interdependent, and sometimes they are not.
For example, for certain subclasses we have a procedure which can guarantee that the entire net is L4, which implies that all transitions are L4.
Checking all transitions individually is therefore unnecessary.
// TODO: Further discussion of the interdependence of these scopes.

Efficiency:
- Use any known procedure, even if it is inefficient.
- Use only efficient procedures if possible, and return `None` if no efficient procedure is known.

For time reasons, I have selectively implemented only the four combinations which appear most useful to me,
with the rest being left as future work.

#grid(
  columns: (auto, auto, auto),
  [Method], [Returns], [Description],
  [`pn.is_live()`], [`bool`], [Check if the entire Petri net is live (L4) or not.],
  [`pn.is_efficiently_live()`], [`Option<bool>`], [Check if the entire Petri net is live (L4) or not using only efficient procedures, returning `None` otherwise.],
  [`pn.liveness()`], [`HashMap<Transition, LivenessLevel>`], [Determine the liveness level of every transition.],
  [`pn.efficient_liveness()`], [`Option<HashMap<Transition, LivenessLevel>>`], [Determine the liveness level of every transition using only efficient procedures, returning `None` otherwise.],
)

Implementations by structural subclass:

Is the entire Petri net live?
#grid(
  columns: (auto, auto, auto),
  [Structural Subclass], [Description of Liveness Procedure], [Is Efficient?],
  [Circuit], [Initial marking contains at least one token], [Yes],
  [State Machine], [Strongly connected and initial marking contains at least one token], [Yes],
  [Marked Graph], [Petri net contains no unmarked circuit], [Yes],
  [Free Choice], [Commoner/Hack Criterion: every siphon contains a marked trap], [No], // TODO: this is currently in the efficient procedures despite being NP-complete.
  [Asymmetric Choice], [CHC sufficient, but not necessary], [No],
  [General], [Construct reachability graph], [No!]
)

A major limitation of the current implementation is the general procedure for checking liveness, which requires constructing the entire reachability graph,
which is prohibitively expensive for all but the smallest bounded nets.
Future work will improve upon this. // TODO: Discuss possibilities.

// TODO: Implementation breakdown for specific liveness levels of all transitions.
