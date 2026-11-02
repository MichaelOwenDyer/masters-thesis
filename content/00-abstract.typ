Petri nets are a standard formalism for verifying process models before they are deployed:
whether a process can get stuck, whether every activity remains possible,
whether resources can accumulate without bound, and whether undesirable states can be reached.
This thesis asks what it takes to much such analysis available in a process-orchestration context,
where programmatic access, memory-safety, portability, and cost control are priorities.
From this setting, the thesis derives eight requirements for a Petri net analyzer,
and a structured survey of 37 tools shows that none of them is fully suitable.

This thesis presents `petrivet`, a Rust library for place/transition Petri nets
with specialized procedures for deadlock-freedom, liveness, boundedness, reachability, and coverability
for nets of restricted structure, and a fallback to general procedures for arbitrary nets.
The caller can restrict the analysis to polynomial-time procedures, or specify an absolute deadline.
Verdicts are accompanied by witnesses or counterexamples, and
the library returns "unknown" when it cannot answer a query.

The evaluation covers 1,976 Petri nets in 115,128 runs.
Of 40,200 verdicts compared with independent reference tools, zero contradictions were found.
Dispatching to specialized procedures by the net's structural class makes the analysis
several hundred times faster than brute-force methods for certain classes of Petri nets,
but preemptive structural checks cost time on more general nets.
Restricting a query to polynomial-time procedures yields answers where brute-force procedures time out,
and deadlines are overshot by less than 50 ms for the median query.
Compared with LoLA, TINA, and ITS-Tools, `petrivet` is competitive on some queries
and slower on others, in particular where there is no specialized procedure for the net's structural class.
The thesis closes with lessons for the design of cost-aware analysis libraries.

#align(bottom, [
  Keywords: Petri nets, behavioral analysis, cost-aware verification, process orchestration, Rust
])
