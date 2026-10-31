#import "./03-research-questions.typ": *

== Methodology

Hevner @hevner_three_2007 visualizes design science as an engine sitting between the problem space and the academic foundations,
forming three interrelated cycles: the Relevance Cycle, the Design Cycle, and the Rigor Cycle.

The Relevance Cycle represents the influence of the real-world domain: the people, organizations, and technologies
exhibiting a concrete problem that the research aims to address.
Consistent engagement with the problem domain ensures that the research is relevant, and that the resulting artifact
alleviates the problem in a meaningful way.

The Rigor Cycle grounds the system design in the theoretical knowledge base, which includes the existing literature,
methodologies, and formalisms that inform the design of the artifact.
The experience gained from the design and evaluation of the artifact feeds back into the knowledge base,
contributing to the academic foundations of the field.

At the center of the model is the Design Cycle, which represents the iterative process of building and
evaluating the artifact. The Design Cycle is informed by the problem domain and the knowledge base,
and in turn informs both of them.

Hevner, March, Park, and Ram @hevner_design_2004 establish seven critical guidelines for conducting design science research.

1. Design as an Artifact: The research must produce a viable artifact.
Hevner et al. define artifacts as constructs (vocabulary/symbols), models (abstractions/representations),
methods (algorithms/practices), or instantiations (implemented systems).

2. Problem Relevance: The research must address a problem that is relevant to the real world.

3. Design Evaluation: The mere construction of an artifact does not constitute research.
The utility, quality, and efficacy of the artifact must be demonstrated by well-executed evaluation methods.

4. Research Contributions: The research must provide clear and verifiable contributions to the existing knowledge base.

5. Research Rigor: The research must apply strict, academically sound methods in both the construction and evaluation phases.

6. Design as a Search Process: The research must acknowledge the iterative nature of the design process within
a search space of possible solutions, while constrained by available resources, computational limits,
and environmental factors.

7. Communication of Research: The research must be presented effectively on both a technical level for other researchers
and a higher level for a broader audience of stakeholders and decision makers.

*Relevance*

The guidelines of Hevner et al. @hevner_design_2004 are addressed in this research as follows:
The artifact (_Design as an Artifact_) is a software instantiation `petrivet`,
together with the methods and design principles that underpin it (@solution-design, @implementation).
Its _Problem Relevance_ is established by the chair's need for an embeddable analyzer (@motivation),
while the survey in @related-work shows that no existing tool completely satisfies this need (#rqs.existing-tools).

The _Design Evaluation_ performed in @evaluation assesses correctness, the benefit of procedure dispatch,
cost control, and the artifact's standing in the context of existing tools (#rqs.correctness to #rqs.comparison).
It also assesses the remaining requirements of @motivation with the same rubric as the survey (@eval-requirements).
The library and the evaluation harness are publicly available, so the results can be checked.
_Research Rigor_ is maintained throughout.
The survey protocol was fixed before the survey was conducted,
the verdicts of `petrivet` were checked against independent reference verdicts,
and threats to validity are discussed for both the survey and the evaluation.

The _Design as a Search Process_ is reflected in the iterative development of `petrivet` and its procedures,
in a search space of possible API designs and algorithms for the backing implementation.
The available resources also limited the search:
the scope was restricted to ordinary Petri nets and the five behavioral properties discussed in @background-behavioral-properties,
while temporal logic and broader Petri net schemas were left for future work.
Finally, the _Communication of Research_ is aimed at two audiences:
this thesis addresses researchers and practitioners in the field of Petri nets,
while the artifact itself is aimed at software engineers.
