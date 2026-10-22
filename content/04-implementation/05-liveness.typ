#import "@preview/lovelace:0.3.0": *
#import "../lib.typ": *
#import "../../lib/tum/elements.typ": *

== Liveness



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
- Check an individual transition.
- Check all transitions.

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

=== Liveness Levels of State Machines

The algorithm makes two linear passes over the condensation graph of the net
(the directed acyclic graph of strongly connected components, or SCCs).
A pass in topological order finds all SCCs reachable from a marked SCC.
A pass in reverse topological order computes for every SCC whether it reaches exactly one terminal SCC
using the following reduction $union.sq$:

$ "One"(a) union.sq "One"(b) := cases("One"(a) & "if" a = b, "Multiple" & "otherwise"), quad "Multiple" union.sq x := "Multiple" =: x union.sq "Multiple". $

Reachable trivial SCCs are $lv(1)$, reachable nontrivial ones $lv(3)$, and a nontrivial SCC that is the unique terminal
SCC reachable from some marked SCC is upgraded to $lv(4)$.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [Liveness levels of a state machine.],
  pseudocode-list(numbered-title: [`StateMachineLiveness`$(N, M_0)$])[
    + $cal(C) <- op("condensation")(G)$; #h(0.5em) $pi <- op("toposort")(cal(C))$
    + $m[K] <- (M_0(K) > 0)$ for all $K in cal(C)$; #h(0.5em) $r <- m$
    + *for each* $K in pi$ *with* $r[K]$:
      + *for each* successor $K'$ of $K$: $r[K'] <- top$
    + *for each* $K in op("rev")(pi)$:
      + *if* $K$ has no successor: $sigma[K] <- "One"(K)$
      + *else* $sigma[K] <- op("succ")(K) pipe op("map")(K' |-> sigma[K']) pipe op("reduce")(union.sq)$
    + $Sigma <- {K' | exists K in cal(C): m[K] and sigma[K] = "One"(K')}$
    + $ell <- cal(C) pipe op("map")(K |-> cases(lv(0) & "if" not r[K], lv(1) & "if" |K| = 1, lv(4) & "if" K in Sigma, lv(3) & "otherwise"))$
    + *return* $cal(C) pipe op("flat_map")(K |-> K pipe op("filter_map")(v |-> (v, ell[K]) "if" v in T))$
  ]
) <algorithm-state-machine-liveness>

A formal proof of correctness is given in @proof-state-machine-liveness.

=== Liveness Levels of Marked Graphs

I present an efficient algorithm (running in $O(|P| + |T| + |F|)$) for computing the liveness levels of all transitions
in a marked graph, based on the work of @murata_petri_1989 @esparza_petri_2026, and @tarjan_depth-first_1972.
To my knowledge this is an original contribution.
The algorithm first sets the liveness level of all transitions to L4.
Then it then identifies all unmarked circuits using Tarjan's SCC algorithm @tarjan_depth-first_1972 on the subnet
induced by the unmarked places and all transitions.
By @murata_petri_1989, all transitions in unmarked circuits are dead (L0).
Then, for each dead transition, the algorithm performs a depth-first search to mark downstream
transitions as L1 if all paths from the dead transitions to them pass through a marked place,
and L0 otherwise.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [Liveness levels of a marked graph.],
  pseudocode-list(numbered-title: [`MarkedGraphLiveness`$(N, M_0)$])[
    + $G_0 <- G[T union {p in P | M_0(p) = 0}]$
    + $Z <-$ #h(0.3em) $op("SCC")(G_0)$
      $pipe op("filter")(|C| > 1)$
      $pipe op("flat_map")(C |-> C)$
      $pipe op("filter_map")(v |-> v "if" v in T)$ #h(1em) // `tarjan_scc` ... `collect`
    + $ell <- (lv(4))_(t in T)$
    + *for each* $z in Z$:
      + $"Dfs"(z, bot)$
    + *return* $op("zip")(T, ell)$
    +
    + *procedure* $"Dfs"(t, b)$:
      + *match* $ell[t]$:
        + $lv(0)$: *return*
        + $lv(1)$ *if* $b$: *return*
        + _otherwise_: $ell[t] <- cases(lv(1) & "if" b, lv(0) & "otherwise")$
      + *for each* $p in post(t)$:
        + ${t'} <- post(p)$
        + $b' <- b or M_0(p) > 0$
        + $"Dfs"(t', b')$
  ]
) <algorithm-marked-graph-liveness>

A formal proof of correctness is given in @proof-marked-graph-liveness.
