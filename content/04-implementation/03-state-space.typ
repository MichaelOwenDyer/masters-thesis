#import "@preview/lovelace:0.3.0": *
#import "../lib.typ": *
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge

// A two-part box: the type name on top, its methods below.
#let class-box(name, methods) = {
  set align(left)
  grid(
    columns: 1,
    inset: (x: 0pt, y: 3pt),
    align(center, strong(raw(name))),
    grid.hline(stroke: 0.5pt),
    ..methods.map(m => text(size: 0.85em, raw(m))),
  )
}

== State Space Exploration

The natural successor to firing transitions manually is the automatic exploration of a Petri net's state space.
The basic idea is to fire all enabled transitions in a marking,
record the resulting markings as children of the current marking (via the fired transitions),
and repeat this process for each newly discovered marking until no new markings can be discovered.
What results is a directed graph of markings connected by transitions.

This procedure is equally applicable to both the reachability graph and the Karp-Miller coverability graph @karp_parallel_1969,
with the only difference being that the coverability graph modifies the markings before recording them,
in a process called _$omega$-acceleration_.
See @background-reachability and @background-coverability for background on the reachability and coverability graphs.

`petrivet` contains a implementation of state space exploration that is generic over the marking type,
allowing it to be shared by the two types of state space graphs.
The implementation allows the user to construct the state space incrementally:
a `StateGraphExplorer` structure (with concrete instantiations `ReachabilityGraphExplorer` and `CoverabilityGraphExplorer`)
represents an in-progress exploration,
which the user can advance cumulatively at their own pace, and stop at any point.
The explorer tracks the "frontier" of the state space and provides methods to advance the exploration by one step,
or to run it to completion,
following either a breadth-first (FIFO) or depth-first (LIFO) strategy.
It also provides methods to search for a marking fulfilling a user-specified predicate,
and accepts an optional deadline to halt exploration, which is particularly useful for infinite state spaces.
The `build_graph()` method runs the exploration to completion
#footnote[or attempts to do so: the documentation heavily warns the user that the reachability graph might be infinite, and so this method might not terminate.]
and returns a `StateGraph` structure
(with concrete instantiations `ReachabilityGraph` and `CoverabilityGraph`)
representing the fully-explored state space,
which offers more advanced methods to query the graph for properties that the given type of state space can answer.

Furthermore, since the coverability graph and the reachability graph coincide for bounded Petri nets,
a fully-explored `CoverabilityGraph` containing no $omega$-markings can be converted into a `ReachabilityGraph`,
to benefit from the larger set of properties that can be answered by the reachability graph.
This pattern is used frequently as `petrivet`'s last-resort method for deciding behavioral properties
on unknown Petri nets: explore the coverability graph, and if we fully construct it without encountering any
$omega$-markings, then we can safely convert it into a reachability graph and use the reachability graph's
more powerful analysis methods.
However, this is at the cost of a nontrivial runtime overhead of $omega$-acceleration,
the implementation of which is shown in @algorithm-omega-acceleration.

#figure(
  {
    set text(size: 0.9em)
    diagram(
      spacing: (1.2em, 6.2em),
      node-stroke: 0.6pt,
      node-inset: 6pt,
      node-corner-radius: 2pt,
      edge-stroke: 0.6pt,
      label-size: 0.8em,
      mark-scale: 80%,

      // reachability on the left, coverability on the right
      node((1, 0), strong(raw("PetriNet")), name: <net>),
      node((0, 1), class-box("ReachabilityGraphExplorer", ("explore_next()", "explore_iter()")), name: <rx>),
      node((2, 1), class-box("CoverabilityGraphExplorer", ("explore_next()", "explore_iter()")), name: <cx>),
      node((0, 2), strong(raw("ReachabilityGraph")), name: <rg>),
      node((2, 2), strong(raw("CoverabilityGraph")), name: <cg>),

      // start an exploration
      edge(<net>, <rx>, "-|>", raw("explore_reachability_graph()"), label-side: right, label-pos: 0.4),
      edge(<net>, <cx>, "-|>", raw("explore_coverability_graph()"), label-side: left, label-pos: 0.4),

      // explore to completion
      edge(<rx>, <rg>, "-|>", raw("build_graph()"), stroke: (dash: "dashed"), label-side: right),
      edge(<cx>, <cg>, "-|>", raw("build_graph()"), label-side: left),

      // shortcuts: explore and build in one step
      edge(<net>, (-1, 0), (-1, 2), <rg>, "-|>", raw("build_reachability_graph()"),
        stroke: (dash: "dashed"), corner-radius: 4pt, label-angle: auto, label-side: right),
      edge(<net>, (3, 0), (3, 2), <cg>, "-|>", raw("build_coverability_graph()"),
        corner-radius: 4pt, label-angle: auto, label-side: left),

      // conversions that succeed only without ω. On Err, the graph or explorer is handed back:
      // each Err loop starts at the "Ok" line of its conversion and ends on the bottom of the box it returns.
      edge(<cg>, <rg>, "-|>", align(left)[#raw("try_into_reachability_graph()") \ `Ok` if no $omega$],
        label-side: right, label-pos: 0.35, label-sep: 0.2em),
      edge((1.17, 2), (1.7, 2.1), "-|>", [`Err`], bend: -60deg, label-side: right),
      edge(<cx>, <rg>, "-|>", align(left)[#raw("try_build_reachability_graph()") \ `Ok` if no $omega$],
        label-side: right, label-angle: auto, label-sep: 0.25em, label-pos: 0.52),
      edge((0.885, 1.576), (1.65, 1.24), "-|>", [`Err`], bend: -60deg, label-side: left),
    )
  },
  caption: [
    The state space API of `petrivet`.
    An explorer is started from a `PetriNet` and advanced incrementally, or run to completion with `build_graph()`.
    Shortcut methods on `PetriNet` do both in one step. \
    Dashed arrows indicate operations which do not terminate for unbounded Petri nets.
    The coverability side converts into a reachability graph only if no $omega$-marking occurs;
    `try_build_reachability_graph()` stops at the first $omega$-marking and returns the explorer,
    so exploration can resume.
  ],
) <fig-state-space-api>

Both explorers share one implementation, shown in @algorithm-state-space-exploration.
The explorer stores the graph discovered so far, a map from each discovered marking to its node in the graph,
and the frontier: a worklist of pairs of a node and a transition that _might be_ enabled at its marking.
Rather than testing every transition at every new marking,
the algorithm adds the postset transitions of the support of the marking ($post(supp(M))$) to the frontier for every newly discovered marking $M$,
plus the ever-enabled source transitions.
$post(supp(M))$ is an over-approximation, since a transition may consume from further, unmarked places,
so the enabling is checked later when the frontier item is popped (or dequeued, depending on depth-first or breadth-first) for exploration.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [State space exploration, shared by the reachability and the coverability graph explorers.],
  pseudocode-list(numbered-title: [`StateGraphExplorer`$(N, M_0)$])[
    + $G <- $ graph with the single node $v_0$ labelled $M_0$
    + $T_"source" <- T pipe op("filter")(t |-> pre(t) = emptyset)$
    + $F <- "Candidates"(M_0) pipe op("map")(t |-> (v_0, t))$
    + $I <- {M_0 |-> v_0}$
    +
    + *procedure* $"ExploreNext"()$:
      + *loop*:
        + *if* $F = emptyset$: *return* $bot$
        + $(v, t) <- op("pop")(F)$
        + *if* $pre(t) subset.eq.not supp(G[v])$: *continue*
        + $M' <- op("fire")(G[v], t)$
        + $"Accelerate"(v, M')$ #h(1em) _(coverability graph only)_#line-label(<algorithm-state-space-exploration-accelerate>)
        + $(v', "new") <- "Register"(v, t, M')$
        + *return* $(t, v', "new")$
    +
    + *procedure* $"Candidates"(M)$:
      + *return* $supp(M) pipe op("flat_map")(p |-> post(p)) union T_"source"$
    +
    + *procedure* $"Register"(v, t, M')$:
      + *if* $M' in op("dom")(I)$:
        + add the edge $v ->^t I[M']$ to $G$
        + *return* $(I[M'], bot)$
      + add a node $v'$ labelled $M'$ and the edge $v ->^t v'$ to $G$; #h(0.5em) $I[M'] <- v'$
      + *for each* $t' in "Candidates"(M')$: #h(0.3em) $op("push")(F, (v', t'))$
      + *return* $(v', top)$
  ]
) <algorithm-state-space-exploration>

The only difference between reachability and coverability graph exploration is @algorithm-state-space-exploration-accelerate:
before registering a new marking, the coverability graph explorer applies $omega$-acceleration to it (@algorithm-omega-acceleration).
It searches backwards through the graph discovered so far for the ancestors of the new node.
If the new marking covers any of its ancestors, unboundedness has been detected:
the new marking enables all the firing sequences that the ancestor marking enabled,
including the one that added the additional tokens,
so that same firing sequence can be repeated indefinitely to produce arbitrarily many tokens
in exactly those places which increased @karp_parallel_1969, @esparza_petri_2026[Thm. 3.2.8].
To avoid this infinite stream of new markings, those places are set to $omega$ to mean arbitrarily large; unbounded.
Then the new marking is registered as usual, and the exploration continues.
@algorithm-omega-acceleration shows pseudocode for this process.

#figure(
  kind: "algorithm",
  supplement: [Algorithm],
  caption: [
    $omega$-acceleration of a discovered marking $M'$ during coverability graph exploration.
  ],
  pseudocode-list(numbered-title: [`Accelerate`$(v, M')$])[
    + $S <- [v]$; #h(0.5em) $V <- emptyset$
    + *while* $S != emptyset$:
      + $u <- op("pop")(S)$
      + *if* $u in V$: *continue*
      + $V <- V union {u}$
      + *if* $G[u] < M'$:
        + *for each* $p in P$ *with* $M'(p) > G[u](p)$: #h(0.3em) $M'(p) <- omega$
      + *for each* edge $u' -> u$ in $G$: #h(0.3em) $op("push")(S, u')$
  ]
) <algorithm-omega-acceleration>

The performance of this algorithm, and an optimization which was applied to it,
is discussed in the section preceding @fig-dispatch-mcc.
