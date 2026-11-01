= Solution Design <solution-design>

This chapter discusses the high-level design of a Petri net analysis tool
which aims to meet the requirements derived in @motivation-requirements.

== Design Goals

The following is a list of informal guiding principles for the design of `petrivet`:

*Avoid unnecessary computation and reuse previously computed results.*
Petri net procedures are generally extremely heavy, and so the tool should not waste resources on unnecessary computations.
If the user did not ask for it, the tool should not compute it.
For the same reason, the tool should be able to reuse previously computed results whenever possible,
or provide a mechanism for the user to do so. There should be no situation where the user's only option is to
recompute a result that has already been computed because of the inflexibility of the tool.

*Decoupled API and implementation.*
The API of the tool and the underlying implementation should be able to evolve independently of each other,
allowing for future extensibility and maintainability.

*Closeness to the theory.*
The tool should be strongly rooted in the theoretical foundations of Petri nets, and its design should be guided by the study of the theory.
Idiomatic formalisms and notations should be used whenever possible, and the tool should not introduce unnecessary
abstractions or concepts that are not present in the theory.
The tool should serve not only as a practical implementation of the theory,
but also as a didactic tool for learning and understanding it.

*Usable for both experts and non-experts.*
The user of the library should not need to have a deep understanding of the underlying theory in order to use the tool effectively.
The tool should pick sensible defaults and provide a simple interface for common use cases,
while still allowing experts to access the full range of functionality.
At the same time, it should do so in a way which is transparent to the user, so that they can understand what is happening and why.

=== API Design

The API a Petri net analysis tool should aim to mirror Petri net theory as closely as possible,
and so its design should be grounded in the study of the theory.
The five behavioral properties discussed in this thesis do not correspond to just one question each.
I identified two dimensions along which the behavioral properties can vary.

*Scope.*
All properties can be checked for the entire Petri net.
Some (boundedness and liveness) can also be checked for individual nodes (places and transitions, respectively).
The user of the library may be interested in one or the other, and so the API should provide a way to
specify the scope of the property check.

*Granularity.*
All properties can be framed in the form of a yes/no question, e.g. "Is this net bounded?" or "Is this transition live?".
But the user may also be interested in the "why" of the answer.
The API should provide a way to specify the desired granularity of the result.

Furthermore, as discussed in @motivation-requirements,
the user may only have a limited amount of time to wait for an answer, and so the API should provide control mechanisms
for the user to prevent the analysis from taking too long.
This is especially important if the user is analyzing a net of unknown structure and size.
Checking a property may be trivial for some nets, but take prohibitively long (effectively forever) for others.
Not being able to predict or control this would be a major inconvenience.

I propose two mechanisms aimed to address this issue, which can be used independently or in combination.

*Wall-clock timeout.*
The user can specify a maximum duration to wait for an answer.
The analysis will be interrupted if it exceeds this time limit, and the analysis will return "unknown".
This mechanism is useful when the user has a hard limit on how long they can wait for an answer,
and is willing to accept that the analysis may not complete in time.

*Efficiency restriction.*
The user can specify that only polynomial-time procedures should be used to check the property.
The analysis will return "unknown" early if no polynomial-time procedure is known for checking the property
for the given Petri net.
This mechanism is useful when the user does not have a hard limit on how long they can wait for an answer,
but just wants to avoid the risk of waiting effectively indefinitely for an answer.
An early "unknown" is preferable to waiting for a time limit,
as the user can immediately re-evaluate whether they want to attempt the analysis again
with a different cost-control mechanism.

Beyond these considerations, it is up to the implementation to determine how best to answer the user's query,
and the implementation should be free to use any combination of procedures with the aim of providing the answer
as quickly and efficiently as possible.

== Functional Requirements

- *FR1.* The tool must provide a native builder API for programmatically constructing Petri net models.
- *FR2.* The tool must be able to parse PNML files into the internal Petri net representation.
- *FR3.* The tool must be able to export Petri nets from the internal representation into PNML files.
- *FR4.* The tool must distinguish between the pure structure ("net") and a net paired with an initial marking ("Petri net").
- *FR5.* The tool must support manual simulation of Petri net models via transition firing.
- *FR6.* The tool must support a variety of significant Petri net structural subclasses:
  Circuit, State Machine, Marked Graph, (Extended) Free-Choice, Asymmetric-Choice, and General nets.
- *FR7.* The tool must provide an API to analyze the fundamental properties
  (k-)boundedness, deadlock-freedom, liveness, coverability, and reachability.
- *FR8.* The tool must support automatic state space exploration and generation of the reachability and Karp-Miller coverability graphs.
- *FR9.* The tool must return evidence with its answers, such as witnesses and counterexamples.
- *FR10.* The tool must return "unknown" when it cannot answer a query, rather than guessing or returning an incorrect answer.
- *FR11.* The tool must provide a mechanism for the user to control the cost of analysis,
  either by specifying a wall-clock timeout or by restricting the analysis to polynomial-time procedures.

== Non-Functional Requirements and Pseudo-Requirements

- *NFR1.* The tool must be implemented in a modern programming language with a strong type system and memory safety guarantees.
- *NFR2.* The tool must be implemented as a library, not a standalone application.