#import "../01-introduction/03-research-questions.typ": *
#import "../eval-data.typ": *

#pagebreak(weak: true)
= Evaluation <evaluation>

This chapter assesses the extent to which `petrivet` fulfills the objectives established in the introduction.
The requirements derived from the chair's needs are listed in @motivation,
and the survey in @related-work applies them to existing tools.
Here I ask whether the library gives correct verdicts, whether its central design decisions pay off,
and how it compares with established tools.
Parts of the evaluation are measured, and parts are assessed against the requirements by inspection and demonstration.
The research questions that concern the artifact are the following (#rqs.existing-tools is answered by the survey in @related-work):

- *#rqs.correctness.* #rqs.correctness-text (@eval-correctness)
- *#rqs.procedure-dispatch.* #rqs.procedure-dispatch-text (@eval-dispatch)
- *#rqs.cost-control.* #rqs.cost-control-text (@eval-cost-control)
- *#rqs.comparison.* #rqs.comparison-text (@eval-comparison)

A hypothesis for each question was established before the evaluation began.
The requirements that cannot be reduced to a measurement, such as the availability of a programmatic interface
or the support for standard model formats, are assessed in @eval-requirements, using the same rubric as the survey.
@eval-threats collects the threats to validity of all parts.

#include "./01-method.typ"
#include "./02-correctness.typ"
#include "./03-procedure-dispatch.typ"
#include "./04-cost-control.typ"
#include "./05-comparison.typ"
#include "./06-requirements.typ"
#include "./07-threats-to-validity.typ"