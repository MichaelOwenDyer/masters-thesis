= Solution Design <solution-design>

== Design Goals

The following is a list of informal guiding principles for the design of petrivet:

- Avoid unnecessary computation and reuse previously computed results.
Petri net procedures are generally extremely heavy, and so the tool should not waste resources on unnecessary computations.
If the user did not ask for it, the tool should not compute it.
For the same reason, the tool should be able to reuse previously computed results whenever possible,
or provide a mechanism for the user to do so. There should be no situation where the user's only option is to
recompute a result that has already been computed because of the inflexibility of the tool.
- Decoupled API and implementation.
The API of the tool and the underlying implementation should be able to evolve independently of each other,
allowing for future extensibility and maintainability.
- Closeness to the theory.
The tool should mirror the theory of Petri nets as faithfully as possible and make frequent references to the theory in its documentation.
Idiomatic formalisms and notations should be used whenever possible, and the tool should not introduce unnecessary
abstractions or concepts that are not present in the theory.
The tool should serve not only as a practical implementation of Petri nets, but also as a didactic tool for learning and understanding the theory.
- Usable for both experts and non-experts.
The user of the library should not need to have a deep understanding of the underlying theory in order to use the tool effectively.
The tool should automatically dispatch to the most efficient procedure available for the net being analyzed.
At the same time, it should do so in a way which is transparent to the user, so that they can understand what is happening and why.

== Functional Requirements

- The tool must provide a native builder API for programmatically constructing Petri net models.
- The tool must be able to parse PNML files into the internal Petri net representation.
- The tool must be able to export Petri nets from the internal representation into PNML files.
- The tool must distinguish between the pure structure ("net") and a net paired with an initial marking ("Petri net").
- The tool must support a variety of significant Petri net structural subclasses: Circuit, State Machine, Marked Graph, (Extended) Free Choice, Asymmetric Choice, and General nets.
- The tool must automatically dispatch to the most efficient procedure based on the detected net subclass.
- The tool must support checking of fundamental properties such as (k-)boundedness, deadlock-freedom, liveness, coverability, and reachability.
- The tool must support manual simulation of Petri net models: transition firing.
- The tool must support automatic state space exploration and generation of the reachability and Karp-Miller coverability graphs.

== Non-Functional Requirements and Pseudo-Requirements

- The tool must be implemented in a modern programming language with a strong type system and support for functional programming paradigms.