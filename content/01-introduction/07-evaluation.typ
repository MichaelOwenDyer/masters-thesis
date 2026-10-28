#import "./03-research-questions.typ": *

== Evaluation

The evaluation consists of a measured component and an assessed component.
The measured component examines the correctness of the answers (#rqs.correctness),
the benefit of dispatching to the cheapest procedure (#rqs.procedure-dispatch),
the ability to control the cost by restricting effort or setting a deadline (#rqs.cost-control),
and the speed and memory use of `petrivet` in comparison with established tools (#rqs.comparison).
The assessed part rates `petrivet` against the requirements of the chair that cannot be reduced to a measurement,
using the same rubric as the survey of existing tools, and reports an integration case study.
The goal is NOT to best existing model checkers, which have had sometimes decades to mature;
this project is a starting point for a new library in the Rust ecosystem,
and in its current state it has no chance of outperforming existing model checkers in several areas.
The comparison is meant to place `petrivet` relative to the state of the art.
I will continue to improve the library over time, adding new procedures and optimizations.
The evaluation is described in detail in @evaluation.

// NOTE (author): example API design evaluation; implement the same thing in other libraries (not part of the current evaluation chapter)
