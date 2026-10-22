= Evaluation <evaluation>

- Oracle on small nets. Generate random nets per subclass (circuit, state machine, marked graph, free-choice, asymmetric, general).
 Compare every structural-theorem result against exhaustive reachability-graph analysis.

- Differential testing against reference tools. Run LoLA, TINA, SMPT and verifypn on the same nets and compare answers.
 The MCC also publishes expected results for its models, which gives you a second oracle.

- Dispatch ablation. For each net class, compare dispatched analysis against forcing the general procedure.
 This is your most direct evidence for RQ2, and it isolates the effect of your design from raw implementation speed.

Comparison setup. Use the same machine, pinned tool versions, equal timeout and memory limits, and repeated runs.
 Report solved counts, PAR-2 and cactus plots, with median and IQR for timings.
 Treat timeouts as censored data and use paired tests (Wilcoxon) for head-to-head comparisons.
 Memory. Record peak RSS and allocation profiles. Per-marking footprint is a useful number if your state space representation is compact.

Rate petrivet on R1–R8 with the same rubric as the survey, with an evidence link for each rating.
 To limit self-assessment bias, have a second person rate it blind. Expect honest P values where they apply (R2, because of the optional Z3 backend).
 Note that forbid(unsafe_code) covers your code only, not Z3.

Integration case study with the chair (the relevance cycle). Integrate into CPEE or a similar orchestration setting,
 using the CLI, the in-process call, or the WASM build.
 Record effort (time, lines of glue code) and what went wrong. This is your naturalistic evidence.

Agent-friendliness, operationalised. Give an LLM agent the documentation and a fixed set of tasks ("is this net live?",
"find the unbounded place"). Compare success rate, errors, and lines of code against one or two other libraries
(SNAKES, libpetri). Fix the model version, repeat each task several times, and report intervals.
 This is novel and cheap, and it turns your wishlist item into data.

