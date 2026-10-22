= Introduction

Petri nets are a well-established mathematical and graphical formalism for modeling concurrent systems,
first introduced by Carl Adam Petri in his 1962 dissertation @petri_kommunikation_1962.
Various properties of Petri nets can offer insights into the behavior of the systems they model,
which can be a powerful tool in the uncovering of design flaws and the verification of domain safety invariants before
a system is deployed @van_der_aalst_putting_1994 @van_der_aalst_workflow_2000 @murata_petri_1989.
While the algorithmic complexity of most Petri net analysis questions is intractable in the general case,
over six decades of Petri net research has produced a wealth of theorems that can be leveraged
to derive efficient decision and semi-decision procedures in broad contexts @murata_petri_1989 @best_petri_2024.
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

A tool that is to be orchestrated this way has requirements beyond analytical power.
It must be callable programmatically, ideally in-process, or at least via a command-line interface with structured input and output.
It must be portable and memory-safe, so that it can be embedded into larger software systems without introducing security vulnerabilities or memory leaks.
It must offer predictable performance and memory usage, so that it can be used in production systems without risk of resource exhaustion.
A query's cost must be predictable enough to be bounded by a caller that cannot wait indefinitely for a result.
It must be extensible, so that new procedures and extensions to the Petri net model can be added as the need arises.
And it should be written with a modern language and toolchain that invite contribution and are legible to AI agents, which increasingly assist in software development.

To my knowledge no existing Petri net analysis tool meets all of these requirements together (see @related-work).
This thesis presents _petrivet_ #footnote[https://github.com/MichaelOwenDyer/petrivet], a Rust library designed to fill this gap.
_petrivet_ implements a variety of decision and semi-decision procedures for Petri net analysis,
including some that are novel to this work.
I evaluate the extent to which _petrivet_ meets the requirements above,
and discuss the design principles that emerged during development.
