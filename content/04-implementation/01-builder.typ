== Builder API

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
