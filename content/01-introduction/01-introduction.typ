#pagebreak(weak: true)
= Introduction

Petri nets are a well-established mathematical and graphical formalism for modeling concurrent systems,
first introduced by Carl Adam Petri in his 1962 dissertation @petri_kommunikation_1962.
Various properties of Petri nets can offer insights into the behavior of the systems they model,
which can be a powerful tool in the uncovering of design flaws and the verification of domain safety invariants before
a system is deployed @van_der_aalst_putting_1994 @van_der_aalst_workflow_2000 @murata_petri_1989.
While the algorithmic complexity of most Petri net analysis questions is intractable in the general case,
over six decades of Petri net research has produced a wealth of theorems that can be leveraged
to derive efficient decision and semi-decision #footnote[A semi-decision procedure is one that sometimes fail to provide an answer, but never provides a wrong answer.]
procedures in broad contexts @murata_petri_1989 @best_petri_2024.
Mature Petri net analysis tools apply these shortcuts instead of the general case procedures where they can @schmidt_lola_2000,
and higher-level tools are built on top of Petri net analyzers to fit more specific use cases, such as business process
model editors @dijkman_formal_2007, verification services, and teaching platforms. // cite specific tools

The Chair of Information Systems at the TU Munich develops infrastructure for business process management,
including the Cloud Process Execution Engine (CPEE) @mangler_cpee_2014 @mangler_cloud_2022,
in which processes orchestrate heterogeneous software services and human actors.
A Petri net analyzer is a natural fit for this domain: process models such as BPMN diagrams @chinosi_bpmn_2012
can be translated into Petri nets, and the properties of those nets (such as boundedness, liveness, deadlock-freedom,
reachability, and coverability) can be used to verify the correctness of the process model @van_der_aalst_workflow_2000
@dijkman_formal_2007.

It is paramount that a Petri net analysis tool is correct: it must give the right answer,
and where it cannot decide, it must say so instead of guessing.
An analyzer that reports a wrong result, or that guesses where it cannot decide,
is actively harmful because it can lead to the deployment of unsafe processes.
Beyond correctness, integration into CPEE and other orchestration platforms yields a set of further requirements
for a Petri net analysis tool:
It must be callable programmatically, ideally in-process, or at least via a command-line interface with structured input and output.
It must be portable and memory-safe, so that it can be embedded without introducing security vulnerabilities or memory leaks.
The cost of a query must be predictable enough to be bounded by a caller that cannot wait indefinitely for a result,
and its time and memory use must be adequate for the models that occur in practice.
It should read and write standard model formats, return evidence with its answers, be extensible
as new procedures and model extensions are needed, and be approachable for the users and contributors
who will build on it, including for students and the AI agents which increasingly assist in software development.

To my knowledge no existing Petri net analysis tool meets all of these requirements together (see @related-work).
This thesis presents `petrivet` #footnote[https://github.com/MichaelOwenDyer/petrivet], a Rust library designed to fill this gap.
Its central design idea is that each question is answered by the cheapest procedure that is applicable to the net at hand.
`petrivet` implements a variety of decision and semi-decision procedures for Petri net analysis,
including some that are novel to this work.
I evaluate whether its answers are correct, whether dispatching to the cheapest procedure pays off,
how well the cost of an analysis can be bounded, and how `petrivet` compares with established tools in speed and memory.
I also assess it against the requirements above and discuss the design principles that emerged during development.

TODO: Mention `petrivet` participated in MCC 2026