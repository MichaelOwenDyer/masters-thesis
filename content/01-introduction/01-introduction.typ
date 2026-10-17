= Introduction

Petri nets are a well-established mathematical and graphical formalism for modeling concurrent systems,
first introduced by Carl Adam Petri in his 1962 dissertation @petri_kommunikation_1962.
Various properties of Petri nets can offer insights into the behavior of the systems they model,
which can be a powerful tool in the uncovering of design flaws and the verification of domain safety invariants before
a system is deployed @van_der_aalst_putting_1994, @van_der_aalst_workflow_2000, @murata_petri_1989.
While the algorithmic complexity of most Petri net analysis questions is intractable in the general case,
over six decades of Petri net research has produced a wealth of theorems that can be leveraged
to derive efficient decision and semi-decision procedures in broad contexts @murata_petri_1989, @best_petri_2024.
Mature Petri net analysis tools apply these shortcuts instead of the general case procedures where they can @schmidt_lola_2000,
and higher-level tools are built on top of Petri net analyzers to fit more specific use cases, such as business process
model editors @dijkman_formal_2007, verification services, and teaching platforms. // cite specific tools
Developers of these higher-level tools need an analyzer that
can be driven programmatically with structured input and output,
runs wherever their software runs without compromising its memory safety,
allows them to anticipate and bound the cost of a query,
supports standard model formats,
performs adequately in time and memory,
and attaches enough theoretical context to each result to derive insight into the domain being modeled.
Existing Petri net analyzers meet these requirements incompletely.
Tool developers ...
I present _petrivet_, a Rust library for modeling place/transition Petri nets and analyzing the five fundamental properties.
It dispatches each query to the cheapest applicable procedure, lets the caller restrict a query to polynomial-time procedures,
and provides a typed interface to the underlying theory.
Its own code contains no unsafe; the optional Z3 SMT solver backend is the one exception.
I evaluate the soundness, traceability, and CPU and memory efficiency of the implementation,
and discuss the design principles that emerged during development.
