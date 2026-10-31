== Petri Net Construction and Simulation <impl-petri-net>

As discussed in @impl-builder, a `NetBuilder` constructs the static `Net` structure.
To simulate dynamic behavior, however, the net must be paired with a marking.
This section introduces the implementation of markings and Petri net simulation in `petrivet`,
which is the basis for behavioral analysis in the library.

=== Tokens and Markings

Tokens, having no intrinsic properties or identity,
are represented as a simple integer count of the number of tokens in each place.
Currently, this integer type is `u32`, which holds a maximum of 4,294,967,295 tokens in a single place,
providing a reasonable upper bound for most practical applications.

Markings have an API representation and an internal representation, each optimized for their respective use cases.

From a user-facing perspective,
it is convenient to only specify the (usually sparse) places that have non-zero token counts in a marking,
leaving the unmentioned places to be implicitly zero.
`petrivet`'s API therefore allows the user to construct a `Marking` from any array or iterator of `(Place, u32)` pairs,
where the `Place` is a handle to a place in the net and the `u32` is the number of tokens in that place.

@fig-impl-marking-api shows an example of constructing a `Marking` from an array of `(Place, u32)` pairs,
and then using that marking to construct a `PetriNet` from a `Net`.

#figure(
  [
    ```
    let mut b = NetBuilder::new();
    let [p0, p1, p2] = b.add_places();
    // ... construct the net ...
    let net = b.build().expect("valid net");
    let marking = Marking::from([(p0, 1), (p2, 3)]);
    let petri_net = PetriNet::new(net, marking);
    // Alternatively, pass the marking array inline
    let petri_net = PetriNet::new(net, [(p0, 1), (p2, 3)]);
    ```
  ],
  caption: [
    A `PetriNet` can be constructed from a `Net` and a `Marking`,
    which can be created from an array of `(Place, u32)` pairs.
    The Petri net has no tokens in `p1`, as it is not mentioned in the marking.
  ]
) <fig-impl-marking-api>

The `Marking` constructor filters, sorts, and deduplicates the input pairs to produce a canonical
representation of the marking, ensuring that equal markings are represented identically regardless
of their construction. This is important for equality comparisons and hashing of markings.

For the core logic, however, a sparse representation is inefficient,
as the token count for a given place is not located at a fixed position in memory.
Instead, `petrivet`'s internal representation of a marking is a dense array of token counts indexed by the canonical place index,
allowing for fast access and manipulation of the marking during simulation and analysis.
This index-based representation is named `IdxMarking` to distinguish it from the user-facing `Marking` type.
The `Marking` passed into the `PetriNet` constructor is converted into an `IdxMarking` for internal use,
while all user-facing methods of `PetriNet` accept and return the more convenient `Marking` type.

The `PetriNet` structure also offers flexibility with respect to the _ownership_ of the underlying `Net` structure,
a unique feature of Rust's type system.
If there is only one marking to be simulated, the `PetriNet` can fully encapsulate the `Net`,
forming a single unified structure that moves as one.
But if the user wishes to simulate multiple initial markings on the same static net structure,
without having to clone the net for each marking,
he or she can also construct a `PetriNet` with a `&Net` reference,
creating a lightweight secondary structure that merely points to the structure of the net without carrying its data.
This is achieved by making the `PetriNet` generic over any type that implements the `AsRef<Net>` trait,
allowing for both owned and borrowed #footnote[and reference-counted, if desired.] nets to be used.

=== Basic Simulation

A `PetriNet` can be used to simulate the behavior of a Petri net by firing transitions and observing the resulting markings.
`PetriNet` is a stateful structure; the marking contained within it is mutable and can be modified by firing transitions.

The `is_enabled()` method can be used to check if a transition is enabled,
and the `enabled_transitions()` method returns an iterator over all currently enabled transitions.

The `fire()` method receives a transition and attempts to fire it, modifying the marking in place if successful
and returning an error if the transition was not enabled.
If the user is already sure that a transition is enabled,
the `fire_unchecked()` method can be used to fire a transition without checking its enabledness,
panicking if the assumption is violated.
For arbitrary firing of any enabled transition, the `fire_any()` method can be used,
which returns the transition that was fired, or `None` if the Petri net is in a deadlock state with no enabled transitions.

`PetriNet` also provides a `is_deadlocked()` helper method to check if the Petri net is in a deadlock state.

@fig-impl-simulation-basic shows an example of simulating a Petri net by firing transitions and observing the resulting markings.

#figure(
  [
    ```
    let mut b = NetBuilder::new();
    let [p0, p1, p2] = b.add_places();
    let [t0, t1] = b.add_transitions();
    b.add_arcs((p0, t0, p1, t1, p2));
    let net = b.build().expect("valid net");
    let mut petri_net = PetriNet::new(net, [(p0, 1)]);
    // t0 is enabled, so it can be fired
    assert!(petri_net.fire(t0).is_ok());
    // t0 is no longer enabled, as it has consumed the only token in p0
    assert!(!petri_net.is_enabled(t0));
    // t1 is now the only enabled transition
    assert_eq!(petri_net.fire_any(), Some(t1));
    // No enabled transitions remain
    assert!(petri_net.is_deadlocked());
    // The final marking has one token in p2 and no others
    assert_eq!(petri_net.marking(), Marking::from([(p2, 1)]));
    ```
  ],
  caption: [
    A `PetriNet` can be used to simulate the behavior of a Petri net by firing transitions and observing the resulting markings.
  ]
) <fig-impl-simulation-basic>