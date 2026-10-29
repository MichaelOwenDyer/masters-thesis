== Research Questions

The objective of this thesis is to design, implement and evaluate an analyzer that meets the requirements of the chair.
It is made precise by five research questions.
The first concerns the problem, and the others concern the artifact, with one question for each of its central claims.

#let rqs = (
  existing-tools: "RQ1",
  existing-tools-text: "To what extent do existing Petri net analysis tools meet the requirements R1 to R8?",
  correctness: "RQ2",
  correctness-text: "Does the implementation give correct answers, and does it return \"unknown\" where it cannot decide?",
  procedure-dispatch: "RQ3",
  procedure-dispatch-text: "How much does answering each question with the cheapest applicable procedure, instead of one general procedure, reduce the cost of analysis?",
  cost-control: "RQ4",
  cost-control-text: "Can a caller effectively bound the cost of an analysis, either by restricting the effort or by setting a deadline?",
  comparison: "RQ5",
  comparison-text: [How does `petrivet` compare with established tools in speed and memory?]
)

- RQ1: #rqs.existing-tools-text
- RQ2: #rqs.correctness-text
- RQ3: #rqs.procedure-dispatch-text
- RQ4: #rqs.cost-control-text
- RQ5: #rqs.comparison-text

#rqs.existing-tools is answered by the survey in @related-work.
#rqs.correctness is answered in @eval-correctness.
The design behind #rqs.procedure-dispatch and #rqs.cost-control is described in @solution-design and @implementation,
and the effect of that design is measured in @eval-dispatch and @eval-cost-control.
#rqs.comparison is answered in @eval-comparison.
The requirements that cannot be reduced to a measurement are assessed in @eval-requirements.
The design principles that emerged during the implementation are discussed in @discussion.
