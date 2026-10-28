
== Threats to validity

- The sample is selected by search and not exhaustive; tools without English or German documentation or without a public page are not included.
- R7 and R8 were added after the survey had begun, as an amendment to the protocol.
  The rating rules were fixed before any data for them was collected.
  They are coarse, observable proxies: the license, the presence of a library interface or documented extension point,
  and five repository indicators. The facts were collected with the help of Claude Sonnet 5.5 from the tools' public pages and repositories,
  at most a few minutes per tool, and not checked by a second rater.
  For five tools (Marcie, Charlie, petritub, ProM/Woflan and Wolfgang) the first pass could not reach or identify the source;
  their ratings rest on a later manual look or default to "not met".
  Commits by AI accounts are not counted as a second author, which lowers the R8 rating of tools maintained by one human with an AI assistant.
- Correctness of the surveyed tools is not rated and is assumed.
  For the contest participants it is supported by their participation in the Model Checking Contest, whose results are cross-checked,
  but for the other tools no evidence was collected.
- R6 and R8 were defined more narrowly than in the first version of the protocol:
  R6 started as "traceability to theory" and was reduced to evidence for answers, and R8 is rated only through contributor indicators.
  The ratings in the table follow the narrower definitions.
- Tools I already knew from prior experience were added by hand, so the selection is biased towards tools familiar to me and, through them, to the model checking community.
- I later apply the same rubric to `petrivet` myself (@eval-requirements), so that rating is not independent of the one given to the surveyed tools,
  and no second rater checked either.
- The scoring is subjective and unverified; I did not ask the authors of the tools to confirm my assessment.
- "Not documented" means that I could not find documentation, not that the tool does not have it.
- Maintenance is judged by the date of the last release or commit, which may not reflect the actual state of the project.
