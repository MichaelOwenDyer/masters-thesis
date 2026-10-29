== State Space Exploration

The natural successor to manual transition firing is automatic state space exploration.
For small, bounded Petri nets, it is feasible to enumerate all reachable markings and construct the reachability graph,
which provides full information about the dynamic behavior of the system.
The reachability graph is a directed graph where each node represents a reachable marking, and each edge represents a
transition firing from one marking to another.

Since the reachability graph is exponentially large in general, it would be inconvenient to only provide a
method to construct the entire graph at once.
Especially for large or unbounded Petri nets, complete construction is infeasible, and the user may only be interested in a small portion of the state space.
Incremental construction is therefore a more flexible approach, allowing the user to explore the state space
cumulatively at their own pace, and to stop exploration at any point.

The `ReachabilityExplorer` structure represents an in-progress exploration of the reachability graph, allowing the user to explore the state space incrementally.
The explorer maintains a "frontier" of transitions enabled in some discovered marking which have not yet been fired, and provides methods to fire these transitions and discover new markings.

The `ReachabilityGraph` structure represents a fully-explored reachability graph, and provides methods to query the graph for properties such as boundedness, deadlock-freedom, and liveness,
which, if we have already done the work of constructing the entire graph, can be answered in linear time by traversing the graph.
The enormous computational cost of constructing the graph in general in the first place cannot be overstated, and so the user should be aware of the potential for long-running computations when calling this method,
or, if the Petri net happens to be unbounded, the graph is infinite and construction will not terminate.

A highly related concept is the Karp-Miller coverability graph, which is identical to the reachability graph for bounded Petri nets,
but provides a finite representation of an infinite state space for unbounded Petri nets by introducing the concept of an "omega" token count,
representing an arbitrarily large number of tokens.

`petrivet` provides analogous structures for the Karp-Miller coverability graph as for the reachability graph,
and the same considerations apply for incremental exploration versus full construction,
with the exception that full construction is guaranteed to terminate.
The size of the graph can still be astronomical.
But if a user has constructed a coverability graph and discovered that no omega markings are present,
they can convert it into a reachability graph.

The Karp-Miller coverability graph comes with a not-insignificant runtime cost, as the algorithm for constructing it
requires comparing every discovered marking with its predecessor markings to determine whether an omega marking should be introduced.

```dot (TODO: fix syntax)
node PetriNet;
node ReachabilityExplorer;
node ReachabilityGraph;
node CoverabilityExplorer;
node CoverabilityGraph;
PetriNet -> ReachabilityExplorer [label="explore_reachability()"];
PetriNet -> CoverabilityExplorer [label="explore_coverability()"];
PetriNet -> ReachabilityGraph [label="build_reachability_graph()"]; // MAY NOT TERMINATE
PetriNet -> CoverabilityGraph [label="build_coverability_graph()"];
ReachabilityExplorer -> ReachabilityExplorer [label=TODO: list of methods for incremental exploration];
ReachabilityExplorer -> ReachabilityGraph [label="build_reachability_graph()"];
CoverabilityExplorer -> CoverabilityExplorer [label=TODO: list of methods for incremental exploration];
CoverabilityExplorer -> CoverabilityGraph [label="build_coverability_graph()"];
CoverabilityGraph -> ReachabilityGraph [label="to_reachability_graph()"];
```

// TODO: Discuss the implementation of these procedures and how the majority of the code is shared

A limitation of the current implementation is that there is no way to construct the set of reachable markings without
also constructing the graph structure, which is a waste of memory if the user only wants to know the markings themselves
and not the transitions between them.
Future work will address this by providing a method to construct the set of reachable markings directly.
