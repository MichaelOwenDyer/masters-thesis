== Simulation

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