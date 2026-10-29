== Contribution

The main contribution of this thesis is `petrivet` #footnote[https://github.com/MichaelOwenDyer/petrivet],
a Rust library that answers each question with the cheapest procedure applicable to the net at hand,
while letting the caller bound the cost.
Its evaluation addresses the correctness of the answers, the effect of dispatching, the control of cost,
and the standing of the library relative to established tools.
I also discuss the design principles that emerged during implementation, and the trade-offs they involved.

analyzer built into a process engine judged by cost control
- structured survey
- open source implementation
- design rests on
  1. recognize before deciding (build time classification)
  2. decide properties by a ladder of procedures
  3. caller controls the cost
  4. return unknown instead of guessing
  5. answers come with evidence - witnesses, counterexamples
  6. internal representation decoupled from api

linear time algorithms with proofs of correctness
evaluation of correctness, dispatch, cost control, and comparison with established tools
design principles for sound cost-aware analysis
  - ordering procedures by worst-case complexity is not the same as ordering by expected cost
  - interface layer should contain no logic
  - scoped to only ordinary petri nets without temporal logic, non goal to be fastest analyzer

should reference chapters where the contribution
