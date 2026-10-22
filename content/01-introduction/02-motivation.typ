== Motivation

Process models are used to communicate and reason about the behavior of a system.
A faulty model can lead to a system that fails in practice, and the cost of such failures can be high. // cite

Some common questions that arise when reasoning about a process model are:

- Can the process ever get stuck in a state from which it cannot proceed?

  *Example*: a service that waits for a human to fill out a form, but the human never receives the form.

- Can the process ever reach a state in which some activity is permanently disabled?

  *Example*: a machine cannot be started because a required part has been removed, and the part cannot be replaced because the machine is not running.

- Can the process accumulate an unbounded amount of resources?

  *Example*: a printer that receives more print jobs than it can handle, and eventually runs out of paper.

- Can the process ever reach a dangerous configuration?

  *Example*: two trains are routed onto the same track, or two perpendicular traffic lights both show green.

Process models can be translated into Petri nets, which provide a mathematical model for reasoning about these questions,
and the analysis results can offer rich insights into the behavior of the system @van_der_aalst_workflow_2000 @murata_petri_1989.
The questions above correspond to the following properties of Petri nets:

- *Deadlock-freedom* (@background-deadlock-freedom) can prove that a process can always proceed, or identify problematic states in which it cannot.
- *Liveness* (@background-liveness) can provide granular insight into the long-term availability of activities in a process.
- *Boundedness* (@background-boundedness) can verify upper limits on the amount of resources a process can accumulate, or identify problematic resource consumption patterns.
- *Reachability* (@background-reachability) and *Coverability* (@background-coverability) can prove the impossibility of undesirable configurations, or provide a concrete example of how they can be reached.

These properties are the reason Petri nets are used in business process management, manufacturing,
and other domains where the correctness of a process is critical.

=== The chair's perspective

The Chair of Information Systems develops the Cloud Process Execution Engine (CPEE) @mangler_cpee_2014 @mangler_cloud_2022
infrastructure, in which processes are assembled from calls to heterogeneous software tools and human actors.
Over time the chair has built up a collection of such tools, and a Petri net analyzer is a
natural addition: a process model's structure can be checked before it runs, by a step in
another process, or by an automated agent assisting a modeler.

Our wishlist for such a tool reads as follows. The analyzer should:

*R1.* be callable programmatically, either in-process or through a command-line interface with structured input and output;

*R2.* be portable and memory-safe, so that it can be embedded into larger software systems without introducing security vulnerabilities or memory leaks;

*R3.* let the caller anticipate and bound the cost of a query, since a process step cannot wait indefinitely;

*R4.* read and write standard model formats;

*R5.* perform adequately in time and memory;

*R6.* provide results with rich context, including counterexamples and witnesses;

*R7.* be extensible, so that new procedures and model extensions can be added as the need arises; and

*R8.* be written in a modern language, with a strong type system and modern tooling, to lower the barrier for contributors, including students and AI agents.

Items 1 to 6 are the requirements assessed in @related-work. The last two describe the
design of the artifact, and I discuss them in @solution-design.

=== The gap

Mature Petri net analyzers exist, and several excel at some of these requirements.
Performance-oriented model checkers lead on R5, workflow tools lead on interchange,
and a few libraries offer a callable interface. The survey in @related-work finds, however,
that no tool meets all of the requirements together, and that the tools which come closest do
so by leaving out the open, extensible foundation the chair wants to build on.
Nobody has, to my knowledge, built the tool that the chair's wishlist describes.
This is the objective of the thesis: to design, implement and evaluate such a tool.

The project was initiated by the chair's need rather than by an observed failure of existing
tools to produce correct answers. In the terminology of Peffers et al. @peffers_design_2007,
it is an objective-centered design science project: the requirements of the problem owner define
the objective, and the contribution is an artifact that meets them.
