Title
Abstract
re-check the "Dispatch by net class" entry of every row against the tool's documentation and source. The column was filled in as a side note when the survey was done, but dispatching by net structure has since become a central claim of the thesis (RQ2), so each entry must be verified before the finding below is relied on.
Related Work Protocol Appendix
Eval tool adapters combining queries like LoLA liveness appendix
Evaluation figures avoid median for performance
Evaluation memory figure avoid symbol overlap
Double check start-up time and memory for each tool and whether the measurement floor was subtracted
Bibliography entries

contribution:
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

