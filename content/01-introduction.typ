= Introduction

Petri nets are a widely used formalism for modeling concurrent systems.
They provide a graphical and mathematical way to represent processes,
making them useful in various domains such as workflow management,
distributed systems, and protocol verification.

== Motivation

Library with a highly ergonomic API
High performance
High correctness
Educational resource for Petri net theory and model checking
Bring Petri net theoretical research into practice

== Research Questions

- What are others doing in this area (model checkers, libraries)?
- How should the tool be implemented?
- How can we evaluate it?

== Contribution

I introduce _petrivet_, a Rust library for modeling and analyzing Petri nets.
The name is a portmanteau of "Petrinet" and "vet", in the sense of "to vet" or "to check" a Petri net model,
or in the sense of a "veterinarian" who checks the health of a Petri net model.

My goal with petrivet is to create _the_ Rust crate for Petri nets.
This means the library should be highly ergonomic, high performance, and correct.
It should also serve as a research and educational resource for Petri net theory and model checking through its documentation and docs.rs.

Petri nets are a well-established formalism for modeling concurrent systems, and they are deceptively simple in their structure.
However, the underlying theory is rich and complex. Petri nets have some of the most difficult to solve yet still decidable problems in computer science.
They can be used to model a wide variety of systems and deliver critical insights into their behavior, which can be a powerful tool
to aid in the design and verification of safety properties before deployment.

Existing petri net model checkers are excellent, but so far the Rust ecosystem has lacked a library that provides
access to Petri net primitives. Rust is a systems programming language that provides strong compile-time guarantees and
is well-suited for implementing high-performance libraries with a focus on correctness. I wanted to bring a high-quality
library to Rust developers and researchers to enable them to dynamically build and analyze Petri net models in a
type-safe and efficient manner.

== Methodology

Design Science in Information Systems (Hevner). How are we doing research?

=== Background

==== Petri Nets

Petri nets, model checking, and fundamental concepts. See @murata_petri_1989.
Define important concepts here:
- structure
- tokens and markings
- transition firing semantics
- reachability and coverability, reachability graph, Karp-Miller coverability graph
- boundedness
- deadlock-freedom
- liveness L0 - L4
- subclasses of nets: circuit, state machine, marked graph, free choice, extended free choice, asymmetric choice, general nets
- traps and siphons, Commoner/Hack Criterion

==== Model Checking

Model checking is a formal verification technique that systematically explores the state space of a system to verify
whether certain properties hold. It involves constructing a model of the system, specifying properties in a formal
language (such as temporal logic), and using algorithms to check if the model satisfies these properties.
Model checking can be applied to various types of systems, including hardware circuits, software programs, and
concurrent systems modeled by Petri nets.

Mention the breadth of model checking and its subtopics:
- LTL and CTL formula checking
- Partial Order Reduction
- Binary Decision Diagrams / Hierarchical Set Decision Diagrams for state space representation
Many of these topics are in the scope of future work for petrivet.
Within the scope of this thesis, we will focus on the fundamental theory of Petri nets and the implementation
of a library that provides a foundation for future work in these areas.

==== Rust Programming Language

==== Design Science in Information Systems

Hevner visualizes design science as an engine sitting between the problem space and the academic foundations. It relies on three primary components:

Environment (The Relevance Cycle): This represents the real-world problem domain. It defines the people, organizations, and technologies that are experiencing a problem. This cycle ensures your research is relevant and actually needed.

IS Research (The Design Cycle): This is the core engine of your thesis. It is an iterative loop of Build (creating the tool) and Evaluate (testing it against the problem requirements).

Knowledge Base (The Rigor Cycle): This represents the existing academic literature, methodologies, formalisms, and architectural patterns you draw upon to build your tool. Your final evaluation contributes new knowledge back to this base.

Hevner defines seven critical guidelines for conducting valid design science research. When you write your methodology section, you will usually map your thesis work directly to these principles to satisfy your chair's requirements.

1. Design as an Artifact: Design science research must produce a viable artifact. Hevner defines artifacts as constructs (vocabulary/symbols), models (abstractions/representations), methods (algorithms/practices), or instantiations (implemented systems). Your implemented tool and its underlying architecture fulfill this requirement.

2. Problem Relevance: The objective is to develop a technology-based solution to an important, unsolved domain problem. This justifies the "Motivation" section of your outline.

3. Design Evaluation: The utility, quality, and efficacy of a design artifact must be rigorously demonstrated. You cannot just build the tool; you must prove it works better, faster, or more safely than existing options. This mandates your qualitative (e.g., developer ergonomics, API design) and quantitative (e.g., benchmarking runtime performance, memory overhead) evaluation.

4. Research Contributions: The project must provide clear and verifiable contributions to the existing knowledge base. This is the exact purpose of the "Contribution" section in your outline—what capability exists now that did not exist before your tool?

5. Research Rigor: You must apply strict, academically sound methods in both the construction and evaluation phases. This means justifying your architectural choices (e.g., leveraging strict compile-time guarantees or specific memory management models) and ensuring your evaluation metrics are reproducible and statistically sound.

6. Design as a Search Process: Finding the right design is an iterative process of discovery. It acknowledges that you are exploring a search space of possible solutions while constrained by available resources, computational limits, and environment laws.

7. Communication of Research: The research must be presented effectively to both highly technical audiences (who need to understand the implementation details) and management-oriented audiences (who need to understand the broader impact).

=== Relevance

Core Artifact and Problem Space
Design as an Artifact: You are engineering a concrete software library (a Rust crate) featuring a builder API, PNML interchange file support, and WebAssembly compilation capabilities.

Problem Relevance: General-purpose model checking algorithms are computationally wasteful when a net possesses specific structural properties. Your tool addresses this by identifying net subclasses upfront (e.g., free choice nets) to deploy significantly more efficient, specific procedures.

Research Contributions: The crate contributes an architectural translation layer that cleanly decouples semantic, user-facing node labels from cache-friendly, numerical IDs optimized for CPU execution. Furthermore, it serves as a highly usable educational resource with extensive documentation.

Engineering Rigor and Evaluation
Research Rigor: Constructing the library in Rust leverages strict compile-time guarantees and strong typing to ensure high performance and correctness. You also enforce rigor by utilizing deterministic algorithmic analysis to categorize nets, explicitly rejecting unpredictable AI heuristics.

Design Evaluation: The efficacy and performance of your tool will be evaluated by its ability to handle brute-force state space enumeration and mitigate the state explosion problem.

Iteration and Presentation
Design as a Search Process: The implementation deliberately starts with the fundamental ordinary Petri net model, consciously deferring syntactic sugar like arc weights and place capacities as future iterative expansions.

Communication of Research: By architecting a strict separation between the static structural net and the dynamic markings, the internal logic remains highly communicable and lightweight. The translation layer guarantees that highly optimized numerical outputs are successfully translated and communicated back to the user via their original semantic names.

== Evaluation

The evaluation of the tool will focus on both qualitative and quantitative aspects.
Qualitative evaluation will assess how well the stated design goals and functional requirements have been met,
including the ergonomics of the API, the clarity of the documentation, and the overall usability of the library.
Quantitative evaluation will measure performance metrics such as execution time, memory usage, and scalability when handling large Petri net models.

The evaluation will also include a comparison with existing Petri net model checkers to evaluate the current status compared to the state of the art.
The goal is NOT to best existing model checkers which have had sometimes decades to mature; this project represents
merely a starting point for a new library in the Rust ecosystem. I will continue to improve the library over time,
adding new procedures and optimizations. It is clear to me that in its current state, the library has no chance of outperforming existing model checkers in several areas.

== Structure


