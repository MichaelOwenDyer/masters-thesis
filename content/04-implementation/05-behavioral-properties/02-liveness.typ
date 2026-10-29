#import "@preview/lovelace:0.3.0": *
#import "../../lib.typ": *
#import "../../../lib/tum/elements.typ": *

== Liveness

This section describes the implementation of the liveness property of Petri nets within `petrivet`.
See @background-liveness for a discussion of the property itself.

Liveness is largely analogous to boundedness in its variability along two dimensions:

*Scope.* Liveness is simultaneously a property of the entire Petri net and of individual nodes (transitions).

*Granularity.* Liveness can be framed as a yes/no question ("is the Petri net #lv[k]-live?"),
but also as a search for the _strict_ liveness level (the maximum $k$ in #lv[k]).

`petrivet` offers two entry points for checking liveness of a `PetriNet`, covering all combinations:

+ `PetriNet::liveness() -> Option<LivenessLevel>` returns the strict liveness level, #lv(0) through #lv(4).
  The user can optionally specify a transition to check, otherwise the entire Petri net is checked.
+ `PetriNet::is_live() -> Option<bool>` corresponds to the yes/no framing.
  In addition to optionally specifying the transition, the user can also specify a minimum liveness level to check for,
  e.g. "is this transition at least #lv(3)-live?". If no level is specified, #lv(4) is assumed.

Furthermore, both methods also accept an optional restriction to only efficient procedures and/or
a timeout to limit the maximum duration of the analysis.

=== Procedure Dispatch <impl-liveness-procedure-dispatch>

`petrivet` descends an array of liveness procedures ordered by increasing computational cost,
checking each in turn until the query is answered or a cost-control mechanism is triggered.
These procedures sometimes yield exact liveness levels, and sometimes only lower bounds
(for example, we have seen a transition fire once, so it is at least #lv(1)-live,
but we cannot yet conclude it is not #lv(2)-live).
Therefore, `petrivet` tracks a lower and upper bound on the true liveness level of each transition,
and can conclude exact level when the lower and upper bounds meet.

*Unmarked Siphon.*
The first procedure checks for the existence of a (maximal) proper siphon unmarked in the Petri net's initial marking.
This is a cheap preliminary check which can immediately prove the transitions in postset of this siphon to be #lv(0).

*State Machines.*
The liveness of transitions in state machines is determined by the structure of the strongly connected components (SCCs)
of the net, and the placement of the tokens within them.
Transitions within an SCC always have the same liveness level.
If there are no tokens in the initial marking, then all transitions are #lv(0).
If there is at least one token, then all transitions are L4-live if the net is strongly connected @esparza_petri_2026[Thm. 5.1.3] @murata_petri_1989[Thm. 4, Thm. 7].
If the net is not strongly connected, then the answer is more nuanced.

The following algorithm computes the liveness levels of all transitions in a state machine in linear time $O(|P| + |T| + |F|)$.
To my knowledge this is an original contribution.
First, the algorithm computes the condensation graph of the state machine using Tarjan's SCC algorithm @tarjan_depth-first_1972.
A pass through the SCCs in topological order finds all SCCs reachable by a token.
A pass in reverse topological order computes for every SCC whether it reaches exactly one terminal SCC
using the following reduction $union.sq$:

$
"One"(a) union.sq "One"(b) := cases(
  "One"(a) "if" a = b,
  "Multiple otherwise"
) \
quad "Multiple" union.sq x := "Multiple" =: x union.sq "Multiple".
$

Reachable trivial SCCs are $lv(1)$, reachable nontrivial ones $lv(3)$,
and a nontrivial SCC that is the unique terminal SCC reachable from some marked SCC is $lv(4)$.

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

*Marked Graphs.*
The liveness of transitions in marked graphs centers around the presence of unmarked circuits
@esparza_petri_2026[Thm. 5.2.3]; @murata_petri_1989[Thm. 7].
The transitions within them are #lv(0)-live (dead), and the transitions downstream of them are at most #lv(1)-live.
All transitions not downstream of an unmarked circuit are #lv(4)-live.

I present a linear-time algorithm ($O(|P| + |T| + |F|)$) for computing the liveness levels of all transitions
in a marked graph, based on the work of @murata_petri_1989 @esparza_petri_2026, and @tarjan_depth-first_1972.
To my knowledge this is an original contribution.
The algorithm first sets the liveness level of all transitions to #lv(4).
Then it then identifies all unmarked circuits using Tarjan's SCC algorithm @tarjan_depth-first_1972
on the subnet induced by all transitions and only the unmarked places.
By @murata_petri_1989, all transitions in unmarked circuits are #lv(0).
Then, for each dead transition, the algorithm performs a depth-first search to mark downstream
transitions as #lv(1) if all paths from the dead transitions to them pass through a marked place,
and #lv(0) otherwise.

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

*Commoner/Hack Criterion.*
For free-choice and asymmetric choice nets,
the Commoner/Hack criterion is sufficient to show that all transitions are #lv(4).
If it does not hold, we know that at least one transition in a free-choice net is not #lv(4),
but not which one, and we know nothing additional for asymmetric choice nets.

*Coverability Graph Exploration.*
As a final fallback, `petrivet` explores the coverability graph, with the hope that the Petri net is bounded so that
the coverability graph is also the reachability graph.
If it is bounded and we do obtain the reachability graph, analyzing its SCCs provides a decision procedure for the
liveness level of every transition:
- a transition which is never fired is #lv(0),
- a transition which appears in all terminal SCCs is #lv(4),
- a transition which appears in any nontrivial SCC is #lv(3), and
- #lv(1) otherwise.
If the Petri net is unbounded, then we halt exploration after the first $omega$-marking is reached,
and can at least conclude that all transitions which have fired so far are #lv(1)-live.

`petrivet` therefore does not have a complete decision procedure for liveness in unbounded Petri nets,
and will invariably return "unknown" for some queries.
Future work may improve this situation, for example by reformulating a liveness query as a reachability query,
and then applying the reachability procedures described in @implementation-reachability-coverability.

TODO: Discuss L2