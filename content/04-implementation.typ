== Implementation

This section discusses the implementation of the petrivet Rust crate, including its architecture, design decisions,
and the rationale behind them.

=== Builder API

The builder API serves as the entry point for constructing Petri net models programmatically.
It allows users to add and remove places, transitions, and arcs, and returns simple handles to these elements with a
backing ascending ID. We leverage Rust's newtype pattern to wrap these IDs in strongly typed handles,
which prevents accidental misuse of the wrong handle type at zero runtime cost.
Arcs are represented in this phase as an enum of either a place-to-transition or transition-to-place arc, which ensures
the bipartite structure of the net is maintained at compile time.
In order to reduce overhead on the user, I provide implementations of the `From` trait for tuples of place and transition
handles, and accept `impl Into<Arc>` for the `add_arc()` method, allowing the user to pass in a tuple of handles directly
without having to construct an `Arc` object explicitly. This enables the user to write `b.add_arc((p, t))` instead of `b.add_arc(Arc::PlaceToTransition(p, t))`,
or `b.add_arc((t, p))` instead of `b.add_arc(Arc::TransitionToPlace(t, p))`, achieving the same effect.

#grid(
  columns: (auto, auto),
  [Operation], [Method on NetBuilder `b`],
  [Add a place], [let p = b.add_place();],
  [Add a transition], [let t = b.add_transition();],
  [Add an arc from place `p` to transition `t`], [b.add_arc((p, t));],
  [Add an arc from transition `t` to place `p`], [b.add_arc((t, p));],
)

We also provide bulk operations for adding multiple places, transitions, or arcs at once, which primarily reduces code bloat for the user.
Here we leverage Rust's powerful type system in several ways.

For bulk adding nodes, we utilize const generics to allow the user to create an arbitrary number of places or transitions ergonomically
in a single operation: the `add_places()` and `add_transitions()` methods return an array of handles of size `N`,
where `N` is a compile-time constant specified by the user or, more conveniently, inferred from context.
The user can call for example `let places = b.add_places::<3>()` to add three places and then access them via `places[0]`, `places[1]`, and `places[2]`,
or preferred: `let [p0, p1, p2] = b.add_places()` to add three places and immediately destructure the returned array into three separate variables.
`N` no longer has to be specified explicitly, as the compiler can infer it from the number of names provided in the destructuring assignment.

For adding multiple arcs at once, I provide multiple possibilities.
I lean on Rust's trait system to provide a single `add_arcs()` method that can accept a variety of input types which can be converted to an Iterator of arcs:
`IntoArcs` is implemented for anything that can be converted into an iterator of individual arcs.
But it is also implemented for tuples of >2 alternating places and transitions, with the semantics that
an arc is created between each consecutive pair of nodes in the tuple.
I find this to be a very ergonomic way to add multiple arcs at once when the user has a specific path in mind, avoiding
the duplication of writing out each individual arc in a list.

#grid(
  columns: (auto, auto, auto),
  [Operation], [Method on NetBuilder `b`], [Note],
  [Add multiple places], [`let [p0, p1, p2] = b.add_places();`], [The user simply names each place in the array destructuring assignment, and the compiler infers the size of the array from the number of names provided.],
  [Add multiple transitions], [`let [t0, t1, t2, t3] = b.add_transitions();`], [],
  [Add multiple arcs], [`b.add_arcs([(p0, t0), (t0, p1), (p1, t1)]);`], [Any type that can be iterated over and converted into an `Arc` can be passed to this method, including arrays, slices, and vectors.],
  [Add multiple arcs - alternate syntax], [`b.add_arcs((p0, t0, p1, t1));`], [This alternate syntax is more compact when the arcs form a contiguous path.],
)

During the builder phase, arcs are stored in a hash map, allowing for fast insertion and removal of arcs.
Once the model is constructed, calling the `build()` method will return a fully constructed, read-only Petri net model that can be analyzed.
On building the net, the builder will reorganize the internal data structures to optimize for cache locality,
packing everything into dense arrays and removing any gaps in the ID space.
Arcs are not stored as separate objects anymore, but instead are represented as adjacency lists of nodes,
the presets and postsets of each node. This allows for fast traversal of the neighborhood of a node.

The `build()` operation also evaluates and caches some structural properties of the net,
such as the net's incidence matrix, its structural subclass (either circuit, state machine, marked graph, (extended) free choice, asymmetric choice, or general)
and whether the net is strongly connected, all of which will be useful for analysis later.
(TODO: section on analysis properties of structural subclasses)

#grid(
  columns: (auto, auto, auto),
  [Petri net element], [Semantic Label], [Dense Internal Representation],
  [Place], [Place(u32)], [PlaceIdx(u32)],
  [Transition], [Transition(u32)], [TransitionIdx(u32)],
  [Arc], [Arc - either Place-Transition or Transition-Place], [Preset and postset adjacency lists],
)

It was an explicit design decision to make NetBuilder mutable and the structure of a built Net immutable, which allows for a clear separation between the construction and analysis phases.
Every place and transition is assigned a unique index in the internal arrays, which allows us to avoid the use of
hash maps and instead use simple array indexing for node-associated data.
This two-tiered representation allows for a decoupling of the user-facing semantic labels of the Petri net components from
the data structure used by the underlying analysis engine. This is a critical design decision that allows freedom to optimize
the internal representation for performance without affecting the user-facing API.

This architecture does not come without overhead.
It necessitates a translation layer that maps the user-facing semantic labels to the internal dense representation and vice versa.
This translation must happen at the beginning and end of every analysis operation, which adds overhead.
Performance overhead is negligible compared to the astronomical computational complexity of some of the problems we are trying to solve.
Development overhead is the more significant concern, as suddenly every core feature must be routed through this translation layer,
which threatens to entangle itself with the core logic of the analysis engine. A development pattern which has slowly emerged
is the separation of the codebase into an API and a core layer, where the API definitions are simple wrappers around the mapping and core data structures.
No logic is implemented in the API layer, it simply maps the user's request to the core layer and maps the results back to the user-facing representation.
This pattern has proven to be a successful strategy for segregating the translation layer from the core logic,
but comes with the cost of additional boilerplate code and a more complex project structure.

=== Tokens and Markings

Up until now, we have only discussed the static structure of a Petri net: the places and transitions and arcs between them.
We now introduce the dynamic aspect of Petri nets, representing the state of the system at a given point.
A _token_ is an abstract representation of a resource or a fulfillment of a condition in the modeled system.
A _marking_ is a distribution of tokens across the places in the Petri net, representing the current state of the system.
Tokens are anonymous and indistinguishable, meaning that the identity of individual tokens does not matter; only the number of tokens in each place is relevant.

Therefore, tokens are represented in petrivet as a simple type alias for `u32`, providing a semantic label for the number without separating it into a distinct type.
Markings have a public representation and an internal representation, each optimized for their respective use cases.
From the API perspective, it is convenient to only specify the places that have tokens, leaving the rest of the places implicitly empty.
Internally, however, we store the number of tokens in each place in a dense array, allowing for fast access and manipulation of the marking.
This dense array has the same size and ordering as the dense array of places, allowing us to zip the two arrays
together and iterate over the places and their corresponding token counts in a single pass.

=== Basic Simulation

The core behavior of a Petri net comes from the transition firing semantics.
Stated loosely, a transition is _enabled_ if all of its input places have at least one token.
An enabled transition may _fire_, which consumes one token from each input place and produces one token in each output place.
Firing one transition may enable or disable other transitions, giving rise to a dynamic system of state changes.

A one-off firing of a transition can be performed by calling the `fire()` method on the `Net` structure object, providing
the transition to fire and a mutable reference to the marking to fire from. The method returns a Rust `Result` containing either the new marking
after firing or an error if the transition was not enabled.
This is a stateless operation in that the `Net` is unchanged, and the marking is not modified in place, but rather a new marking is returned.
This is not always what we want, and becomes cumbersome when we want to fire a sequence of transitions,
as we must keep track of the current marking and pass it to each subsequent firing operation.

We therefore introduce a stateful `PetriNet` structure which combines a `Net` and an initial `Marking`,
allowing for the simulation and mutation of the Petri net state over time.
We can then call the `fire()` method on the `PetriNet` structure, which will attempt to fire the given transition from the current marking,
modifying the marking in place if successful, or returning an error if the transition was not enabled.

petrivet provides three different ways to fire transitions, each for a different use case.

#grid(
  columns: (auto, auto, auto),
  [Operation], [Returns], [Use Case],
  [`pn.is_enabled(t)`], [`bool`], [Check if a particular transition is enabled.],
  [`pn.fire(t)`], [`Result<(), NotEnabled>`], [Check if a particular transition is enabled, and fire it if so, or leave the state unchanged otherwise.],
  [`pn.fire_any()`], [`Option<Transition>`], [Fire an arbitrary enabled transition, or determine that the Petri net is in a deadlock state.],
  [`pn.fire_unchecked(t)`], [`()`], [Fire a particular transition when the user is already sure that it is enabled.]
)

=== State Space Exploration

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
The enormous computational cost of constructing the graph in general in the first place cannot be understated, and so the user should be aware of the potential for long-running computations when calling this method,
or, if the Petri net happens to be unbounded, the graph is infinite and construction will not terminate.

A highly related concept is the Karp-Miller coverability graph, which is identical to the reachability graph for bounded Petri nets,
but provides a finite representation of an infinite state space for unbounded Petri nets by introducing the concept of an "omega" token count,
representing an arbitrarily large number of tokens.

petrivet provides analogous structures for the Karp-Miller coverability graph as for the reachability graph,
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

=== Fundamental Property Checking

The fundamental properties of interest for Petri nets are:
- boundedness,
- deadlock-freedom,
- liveness,
- coverability, and
- reachability.

petrivet provides methods to check each of these properties.
At this point, we would be missing a major opportunity if we did not consider the structural subclass of the net when checking these properties,
as the computational complexity varies significantly depending on the subclass.
petrivet dispatches to the most efficient known procedure available for the net on a best-effort basis.
When the net is a subclass for which no efficient procedure is known, petrivet will fall back to the general procedure for arbitrary nets.
This may not be ideal for all users, especially when analyzing nets of unknown magnitude, as the general procedures are often prohibitively expensive.
Therefore petrivet also provides incomplete methods for checking these properties using only efficient procedures, insofar as they are known and implemented,
and will return `None` if no efficient procedure is known for the net.
This gives the user the choice to check for these properties quickly if possible, or to check for them exhaustively
if they are willing to pay the computational cost in the general case.

An overview of the procedures implemented in petrivet for checking these properties is provided in the following sections.

Each of these properties is implemented as a ladder of increasingly expensive procedures,
such that the cheapest procedures are attempted first, and the more expensive procedures are only attempted if the
cheaper ones fail to provide an answer. Some procedures are complete, meaning they will always return a definitive answer,
while others are incomplete, particularly for unbounded nets when the final fallback procedure is to construct the reachability graph.
More work is needed to provide complete procedures for unbounded nets, and this is a major area of future work for petrivet.

petrivet also provides several variants of each property check across three dimensions.
+ Desired granularity of the result
+ Scope (individual place / transition, or entire net)
+ Efficiency restrictions (use any known procedure, or only procedures which run in polynomial time, returning `None` if no such procedure is known)

==== Theorems

#grid(
  columns: (auto, auto, auto, auto, auto, auto, auto, auto),
  [Applicability], [Hypothesis], [Direction], [Condition], [Theoretical Complexity], [Citation], [Implementation status], [Description],
  [Circuit], [Conservative], [<=>], [True], [O(1)], [], [Implemented], [Circuits are conservative],
  [State Machine], [Conservative], [<=>], [True], [O(1)], [], [Implemented], [State machines are conservative],
  [Live Marked Graph], [Bounded], [<=>], [Strongly Connected], [], [], [Implemented], [A live marked graph is bounded iff it is strongly connected],
  [Live Free Choice], [Bounded], [<=>], [Every place belongs to an S-Component], [], [Heck's Theorem], [Not Implemented], [A live free choice net is bounded iff every place belongs to an S-Component],
  [General], [Bounded], [<=], [Positive place subvariant exists], [], [], [], [],
)

==== (K-)Boundedness

A place is said to be _k-bounded_ for some finite integer `k` if the place never
contains more than `k` tokens in any reachable marking.
A place is _bounded_ if it is k-bounded for some finite `k`, and _unbounded_ otherwise.
The Petri net as a whole is said to be (k-)bounded if all of its places are (k-)bounded,
and unbounded if at least one of its places is unbounded.

We represent this property in Rust as an enum:
```rust
pub enum Boundedness {
  Bounded(K),
  Unbounded,
}
```
where `K` is a type alias for `u32`, representing the smallest `k` for which the place (or Petri net) is k-bounded (on a best-effort basis).

I identify three dimensions along which we can classify the problem of checking (k-)boundedness:

Operation:
- Check whether bounded or unbounded -> bool.
- Like above, but additionally find the minimal `k` in the bounded case -> Boundedness.

Scope:
- Check an individual place.
- Check the entire net.

Efficiency:
- Use any known procedure, even if it is inefficient.
- Use only efficient procedures if possible, and return `None` if no efficient procedure is known.

This gives us a total of 8 different procedures for checking (k-)boundedness,
which are implemented in petrivet as methods on the `PetriNet` structure.

#grid(
  columns: (auto, auto, auto),
  [Method], [Returns], [Description],
  [`pn.is_bounded()`], [`bool`], [Check if the Petri net is bounded or unbounded.],
  [`pn.is_place_bounded(p)`], [`bool`], [Check if the Place `p` is bounded or unbounded.],
  [`pn.is_efficiently_bounded()`], [`Option<bool>`], [Check if the Petri net is bounded or unbounded using only efficient procedures, returning `None` otherwise.],
  [`pn.is_place_efficiently_bounded(p)`], [`Option<bool>`], [Check if the Place `p` is bounded or unbounded using only efficient procedures, returning `None` otherwise.],
  [`pn.boundedness()`], [`Boundedness`], [Check the k-boundedness of the Petri net.],
  [`pn.place_boundedness(p)`], [`Boundedness`], [Check the k-boundedness of a particular place `p`.],
  [`pn.is_efficiently_boundedness()`], [`Option<Boundedness>`], [Check the k-boundedness of the Petri net using only efficient procedures, returning `None` otherwise.],
  [`pn.is_place_efficiently_boundedness(p)`], [`Option<Boundedness>`], [Check the k-boundedness of a particular place `p` using only efficient procedures, returning `None` otherwise.],
)

TODO: Discuss the implementation of these procedures broken down by net structure.

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

=== Deadlock-Freedom

A Petri net is said to be _deadlock-free_ if, from every reachable marking, there exists at least one enabled transition;
that is, there is no reachable marking from which the system cannot proceed further.

Deadlock-freedom is EXPSPACE-complete in general, but there are efficient procedures for certain subclasses of nets.

petrivet's API provides the following methods for checking deadlock-freedom:

#grid(
  columns: (auto, auto, auto),
  [Method], [Returns], [Description],
  [`pn.is_deadlock_free()`], [`bool`], [Returns true if the Petri net is deadlock-free and false otherwise.],
  [`pn.is_efficiently_deadlock_free()`], [`Option<bool>`], [Returns true if the Petri net is deadlock-free and false otherwise, using only efficient procedures if possible, returning `None` otherwise.],
  [`pn.deadlocks()`], [`Iterator<Deadlock>`], [Return an iterator over all reachable deadlock markings, if any exist.],
)

=== Reachability & Coverability

=== Commoner Hack Criterion