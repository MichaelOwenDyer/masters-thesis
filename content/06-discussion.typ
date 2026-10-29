#pagebreak(weak: true)
= Discussion <discussion>

Discuss evaluation findings, revisit the research questions, and summarize strengths and weaknesses of the solution and implementation.

== Implementation Discussion

Early versions of `petrivet` had no distinction between the user-facing API and the internal core logic.
The user directly interacted with internal data structures and called internal procedures.
This was a simple and straightforward design, but it had several drawbacks.

The data the analysis algorithms needed was not always in the same format as the data the user wanted to see.

The performance of the analysis was not consistent and depended on the exact sequence of
operations the user performed to construct the net.

The builder API is designed to be ergonomic and safe,
leveraging Rust's type system to prevent common mistakes at compile time.
The internal representation of the net is optimized for analysis,
with a dense, cache-friendly layout that lends itself well to analysis algorithms.
The decoupling of the API and the core logic allows for flexibility developing both layers independently.
However, this decoupled architecture does not come without overhead.
It necessitates a translation layer that maps the user-facing semantic labels to the internal dense representation
and vice versa.
This translation must happen at the beginning and end of every analysis operation,
and must not be allowed to entangle itself within the hot path of the core logic,
as that would introduce unnecessary overhead and complexity.
Maintaining this separation of concerns is a constant development challenge, and requires careful design and discipline:
every feature of the library has two sides, the user-facing API and the internal core logic,
and maintaining a clean separation between the two is crucial for the long-term maintainability of the codebase.

A development pattern which has slowly emerged (but not yet to its full extent)
is the separation of the codebase into an API layer and a core layer,
where the core layer is not allowed to access the API layer,
and the API layer can only access the core layer to call its analysis functions
and map the results back to the user-facing data structures.
This pattern has proven to be a successful strategy for segregating the translation layer from the core logic,
but comes with the cost of additional boilerplate code and a more complex project structure.

