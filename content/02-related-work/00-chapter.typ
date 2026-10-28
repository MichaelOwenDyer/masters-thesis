#import "../01-introduction/03-research-questions.typ": *

#pagebreak(weak: true)
= Related Work <related-work>

This chapter answers #rqs.existing-tools: to what extent do existing Petri net analysis tools meet the requirements
R1 to R8 derived in @motivation?

These are:
- programmatic access (R1),
- memory-safe portability (R2),
- cost control (R3),
- model interchange (R4),
- performance and memory efficiency (R5),
- evidence with answers (R6),
- extensibility (R7), and
- approachability (R8).

Correctness is a precondition for any tool, but it is not evaluated here.
See @eval-correctness for a correctness evaluation of `petrivet` compared to a small set of established tools.

I first describe the methodology used to select the tools to survey,
and then present the 37 selected tools with regard to their support for the requirements.

Beyond the ratings, I look at one more feature that matters for #rqs.procedure-dispatch:
whether a tool documents that it chooses its procedure by the structure of the net.

Finally, I state the conclusions drawn from the survey and state the gap that `petrivet` aims to fill.

#include "./01-methodology.typ"
#include "./02-survey.typ"
#include "./03-threats-to-validity.typ"