== NetBuilder <impl-builder>

The builder API serves as `petrivet`'s entry point for programmatically defining the structure of a net:
places, transitions, and the arcs between them (FR1).

A builder can be created with `NetBuilder::new()`,
and places and transitions can be added with `add_place()` and `add_transition()`.
These methods return opaque handles to the created nodes,
which are wrapper types (_newtypes_) around ascending integer IDs.
Rust allows distinct concepts to have distinct types at no additional runtime cost,
preventing accidental misuse of the API at compile time,
such as passing a place where a transition is expected.

Arcs are represented as an enum: either a place-to-transition or transition-to-place arc,
guaranteeing the bipartite structure of the net by refusing to compile otherwise @fig-impl-arc-enum.

#figure(
  [
    ```
    enum Arc {
        PlaceToTransition(Place, Transition),
        TransitionToPlace(Transition, Place),
    }
    ```
  ],
  caption: [
    The `Arc` enum guarantees the bipartite structure of the net at compile time.
  ]
) <fig-impl-arc-enum>

To improve API ergonomics, I provide implementations of Rust's `From` trait
for the tuple types `(Place, Transition)` and `(Transition, Place)` to be converted into an `Arc`,
which works well with Rust's `Into` trait to allow the user to specify an arc as a pair of nodes
@fig-impl-builder-basic.

#figure(
  [
    ```
    let mut b = NetBuilder::new(); // Create a new builder
    let p = b.add_place(); // Add a place
    let t = b.add_transition(); // Add a transition
    b.add_arc((p, t)); // Add an arc from place `p` to transition `t`
    b.add_arc((t, p)); // Add an arc from transition `t` to place `p`
    ```
  ],
  caption: [
    The builder API allows the user to add and remove places, transitions, and arcs,
    and returns opaque handles to the created nodes.
    The `add_arc()` method accepts any type that can be converted into an `Arc`,
    including tuples of opposite node types, which are semantically interpreted as arcs.
  ]
) <fig-impl-builder-basic>

Removing a node or an arc is as simple as calling the corresponding `remove_*()` method with the handle
of the element to be removed.
All adjacent arcs are automatically removed as well, and the removed node's internal ID is not reused to ensure
that the invalidated handles do not accidentally refer to a new node that has been added later.

`petrivet`'s API also provides bulk operations for adding multiple places, transitions, or arcs at once,
with the goal of reducing boilerplate code.
Rust's type system can be leveraged to provide ergonomic syntax here as well.

For adding several nodes at once, _const generics_ and _array destructuring_ allow the user to create an arbitrary number of
places or transitions at once without having to explicitly specify the desired number (@fig-impl-bulk-nodes).

#figure(
  [
    ```
    let mut b = NetBuilder::new();
    // Add three places
    let [p0, p1, p2] = b.add_places();
    // Add four transitions
    let [t0, t1, t2, t3] = b.add_transitions();
    ```
  ],
  caption: [
    Bulk-adding of places and transitions is supported via const generics,
    allowing the user to add an arbitrary number of nodes at once.
  ]
) <fig-impl-bulk-nodes>

For adding multiple arcs at once, the `add_arcs()` method accepts a tuple of 3-12 alternating places and transitions
#footnote[The maximum length of 12 was chosen arbitrarily, and could be increased if desired. Rust's lack of variadic generics prevents a more general solution.].
These are semantically interpreted as a path through the net,
and an arc is added between each neighboring pair of nodes in the path (@fig-impl-add-arcs).

#figure(
  [
    ```
    let mut b = NetBuilder::new();
    let [p0, p1, p2] = b.add_places();
    let [t0, t1] = b.add_transitions();
    // Add three arcs using path syntax
    b.add_arcs((p0, t0, p1, t1));
    // This will fail to compile, as it is not a valid path
    !{b.add_arcs((t0, t1, p2))};
    ```
  ],
  caption: [
    `petrivet`'s `add_arcs()` builder method allows adding multiple arcs at once by
    specifying a path through the net as a tuple of alternating places and transitions.
    Rust's type system allows flexibility without sacrificing correctness.
  ]
) <fig-impl-add-arcs>

The `NetBuilder` is optimized for mutability, allowing the user to add and remove nodes and arcs in any order.
Once construction is complete, the user can call the `build()` method to finalize the net and return a read-only
`Net` object.

Internally, `build()` transforms the internal mutation-oriented data structures
into a densely packed representation optimized for analysis.
- Every place and transition is assigned a unique index in the internal arrays,
  removing any gaps in the ID space which might have arisen via removals of nodes during construction.
  This creates a canonicalized internal layout decoupled from the API construction order,
  allowing for optimizations such as sorting the nodes by their degree or other properties to improve cache locality.
  This also allows any data associated with places or transitions to be stored in a CPU cache-friendly contiguous array
  in the canonical order (as opposed to a hash map or other associative data structure).
  Sets of places or transitions can be represented as bitsets, which is especially data-dense.
  A mapping is maintained between the user-facing semantic labels of the nodes and their internal canonical indices,
  allowing the user to continue to refer to nodes by their original handles.
- Arcs are no longer represented as separate objects, but rather as adjacency lists of node indices.
  This allows for cache-friendly iteration over the preset and postset of a node,
  a common operation in many analysis algorithms.
- The structural class of the net is evaluated once and cached, along with the net's incidence matrix
  and whether the net is strongly connected.
  All of these properties will be useful for analysis later, and since the net is immutable,
  will not need to be recomputed for the lifetime of the net.

The `build()` method additionally performs two validation checks on the net:
- The net must be connected.
  A non-connected net, in the opinion of the author, is not a net at all, but rather a collection of nets.
- The net must contain at least one place and one transition.
  A single place, a single transition, or an empty net is a degenerate case with no interesting properties.

If either of these checks fails, the `build()` method returns a `NetError` enum indicating the reason for failure.
Otherwise, it returns a fully constructed, read-only `Net` object that is ready to be analyzed @fig-impl-build-result.

#figure(
  [
    ```
    let mut b = NetBuilder::new();
    // ... construct the net ...
    let net = match b.build() {
        Ok(net) => net, // The net is valid and can be analyzed
        Err(NetError::NotConnected) => panic!("The net is not connected"),
        Err(NetError::Degenerate) => panic!("The net must contain at least one place and one transition"),
    };
    // Alternatively, use the `?` operator to propagate the error
    let net = b.build()?;
    ```
  ],
  caption: [
    The `build()` method returns a `Result<Net, NetError>`, which is either a fully constructed,
    read-only net model that can be analyzed,
    or a `NetError` if the net is not connected or degenerate (i.e. contains no places or no transitions).
  ]
) <fig-impl-build-result>

The user can check the net's structural class (FR6) and whether it is strongly connected
via the `class()` and `is_strongly_connected()` methods, respectively (@fig-impl-query-class)

#figure(
  [
    ```
    let mut b = NetBuilder::new();
    let [p0, p1] = b.add_places();
    let t0 = b.add_transition();
    b.add_arcs((p0, t0, p1));
    let net = b.build()?;
    assert_eq!(net.class(), NetClass::StateMachine);
    assert_eq!(net.is_strongly_connected(), false);
    ```
  ],
  caption: [
    The `class()` and `is_strongly_connected()` methods allow the user to query the net's structural class
    and whether it is strongly connected.
  ]
) <fig-impl-query-class>

If the user wishes to modify the net after construction,
they can convert a `Net` back into a `NetBuilder` and continue to add or remove nodes and arcs.
Building the net again will re-evaluate the canonical internal representation and the validation checks,
ensuring that the unique story of how a net was constructed does not affect its analysis properties.

#figure(
  [
    ```
    // ... construct the net ...
    let net = b.build()?;
    // Convert the net back into a builder to modify it
    let mut b2 = NetBuilder::from(net);
    let t1 = b2.add_transition();
    b2.add_arc((p1, t1));
    let net2 = b2.build()?;
    ```
  ],
  caption: [
    A `Net` can be converted back into a `NetBuilder` to allow further modifications.
    The canonical internal representation and validation checks are re-evaluated when the net is built again.
  ]
) <fig-impl-net-into-builder>
