#import "../01-introduction/03-research-questions.typ": *
#import "../eval-data.typ": *

== Comparison with established tools <eval-comparison>

#rqs.comparison asks how `petrivet` compares with established tools in speed and memory.
The comparison is performed with LoLA, TINA, and ITS-Tools,
and is intended to place `petrivet` in the landscape of existing tools
rather than to rank the tools against each other.

#let e4 = rows("e4.csv")                        // solved questions per collection, question and tool
#let time-h2h = rows("e4b_time_headtohead.csv")   // petrivet against LoLA and TINA, times, per collection and question
#let mem-h2h = rows("e4b_memory_headtohead.csv")  // the same for the peak memory
#let frontier = rows("e4b_frontier.csv")        // largest size at which a tool answers all four questions of a family
#let frontier-families = frontier.map(r => r.family).dedup()
#let startup = rows("e9_startup.csv")
#let bytes = rows("e7_bytes_per_state.csv")

*Queries answered.*
@tab-solved-fam, @tab-solved-mcc-small and @tab-solved-mcc-large show how many queries each tool answers within the
30 second time limit, per property and collection.
Under each count is the penalized average time (PAR-2) in seconds,
in which a timeout, memout, or crash counts as twice the time limit;
"unknown" results returned before the limit count as the time it took.
A dash means that the tool has no procedure for the query.
No counted verdict of any tool contradicts the reference verdict;
the exception is unbounded deadlock-freedom, explicitly out of scope of the comparison,
where ITS-Tools gives incorrect verdicts (@eval-correctness).

#figure(
  eval-table(
    (2.2fr, 1fr, 1fr, 1fr, 1fr),
    ("Tool", question-name.at("Q1"), question-name.at("Q2"), question-name.at("Q3"), question-name.at("Q4")),
    ..("pv-dispatch", "pv-general", "pv-poly", "lola", "tina", "its-tools").map(tool => (
      [#solver-name.at(tool)],
      ..("Q1", "Q2", "Q3", "Q4").map(q => {
        // the row of this tool and question in e4.csv
        let r = e4.find(row => row.source == "fam" and row.query == q and row.solver_id == tool)
        if int(r.n_supported) == 0 {
          [–]  // the tool has no method for this question
        } else {
          [#r.n_solved / #r.n_cells \ #text(size: 6.5pt, fill: luma(100))[#dec(float(r.par2_ms) / 1000, d: 1) s]]
        }
      }),
    )).flatten(),
    align: (left, right, right, right, right),
  ),
  caption: [
    Queries answered by each tool on the scaling families (144 per property),
    and the penalized average time in seconds under each count.
  ],
) <tab-solved-fam>

On the parametrized families, `petrivet` dispatch answers all #e4.find(r => r.source == "fam" and r.query == "Q2" and r.solver_id == "pv-dispatch").n_solved
boundedness queries (LoLA #e4.find(r => r.source == "fam" and r.query == "Q2" and r.solver_id == "lola").n_solved, TINA #e4.find(r => r.source == "fam" and r.query == "Q2" and r.solver_id == "tina").n_solved),
#e4.find(r => r.source == "fam" and r.query == "Q3" and r.solver_id == "pv-dispatch").n_solved
liveness queries (LoLA #e4.find(r => r.source == "fam" and r.query == "Q3" and r.solver_id == "lola").n_solved, TINA #e4.find(r => r.source == "fam" and r.query == "Q3" and r.solver_id == "tina").n_solved)
and #e4.find(r => r.source == "fam" and r.query == "Q1" and r.solver_id == "pv-dispatch").n_solved
deadlock-freedom queries, as many as LoLA and more than TINA (#e4.find(r => r.source == "fam" and r.query == "Q1" and r.solver_id == "tina").n_solved).
LoLA answers more quasi-liveness queries (#e4.find(r => r.source == "fam" and r.query == "Q4" and r.solver_id == "lola").n_solved
against #e4.find(r => r.source == "fam" and r.query == "Q4" and r.solver_id == "pv-dispatch").n_solved).

#figure(
  eval-table(
    (2.2fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr),
    ("Tool", question-name.at("Q0"), question-name.at("Q1"), question-name.at("Q3"), question-name.at("Q4"), question-name.at("Q5"), question-name.at("Q6")),
    ..("pv-dispatch", "pv-general", "pv-poly", "lola", "tina", "its-tools").map(tool => (
      [#solver-name.at(tool)],
      ..("Q0", "Q1", "Q3", "Q4", "Q5", "Q6").map(q => {
        // the row of this tool and question in e4.csv
        let r = e4.find(row => row.source == "mcc-small" and row.query == q and row.solver_id == tool)
        if int(r.n_supported) == 0 {
          [–]  // the tool has no method for this question
        } else {
          [#r.n_solved / #r.n_cells \ #text(size: 6.5pt, fill: luma(100))[#dec(float(r.par2_ms) / 1000, d: 1) s]]
        }
      }),
    )).flatten(),
    align: (left, right, right, right, right, right, right),
  ),
  caption: [
    Queries answered by each tool on the 90 small competition models,
    and the penalized average time in seconds under each count.
  ],
) <tab-solved-mcc-small>

On the small MCC models, LoLA and TINA answer more deadlock-freedom queries
(#e4.find(r => r.source == "mcc-small" and r.query == "Q1" and r.solver_id == "lola").n_solved and #e4.find(r => r.source == "mcc-small" and r.query == "Q1" and r.solver_id == "tina").n_solved)
than `petrivet` dispatch (#e4.find(r => r.source == "mcc-small" and r.query == "Q1" and r.solver_id == "pv-dispatch").n_solved),
and more quasi-liveness queries (#e4.find(r => r.source == "mcc-small" and r.query == "Q4" and r.solver_id == "lola").n_solved and #e4.find(r => r.source == "mcc-small" and r.query == "Q4" and r.solver_id == "tina").n_solved
against #e4.find(r => r.source == "mcc-small" and r.query == "Q4" and r.solver_id == "pv-dispatch").n_solved).
For liveness, `petrivet` dispatch (#e4.find(r => r.source == "mcc-small" and r.query == "Q3" and r.solver_id == "pv-dispatch").n_solved)
is ahead of LoLA (#e4.find(r => r.source == "mcc-small" and r.query == "Q3" and r.solver_id == "lola").n_solved)
and behind TINA (#e4.find(r => r.source == "mcc-small" and r.query == "Q3" and r.solver_id == "tina").n_solved),
and for 1-safety all tools are within a few models of each other.

#figure(
  eval-table(
    (2.2fr, 1fr, 1fr, 1fr, 1fr),
    ("Tool", question-name.at("Q1"), question-name.at("Q3"), question-name.at("Q4"), question-name.at("Q5")),
    ..("pv-dispatch", "pv-general", "pv-poly", "lola", "tina", "its-tools").map(tool => (
      [#solver-name.at(tool)],
      ..("Q1", "Q3", "Q4", "Q5").map(q => {
        // the row of this tool and question in e4.csv
        let r = e4.find(row => row.source == "mcc-large" and row.query == q and row.solver_id == tool)
        if int(r.n_supported) == 0 {
          [–]  // the tool has no method for this question
        } else {
          [#r.n_solved / #r.n_cells \ #text(size: 6.5pt, fill: luma(100))[#dec(float(r.par2_ms) / 1000, d: 1) s]]
        }
      }),
    )).flatten(),
    align: (left, right, right, right, right),
  ),
  caption: [
    Queries answered by each tool on the 20 large competition models,
    and the penalized average time in seconds under each count.
  ],
) <tab-solved-mcc-large>

On the large MCC models, `petrivet` dispatch answers #e4.find(r => r.source == "mcc-large" and r.query == "Q1" and r.solver_id == "pv-dispatch").n_solved
of #e4.find(r => r.source == "mcc-large" and r.query == "Q1" and r.solver_id == "lola").n_cells
deadlock-freedom queries where LoLA and ITS-Tools answer #e4.find(r => r.source == "mcc-large" and r.query == "Q1" and r.solver_id == "lola").n_solved,
and TINA none; these are 20 models, so the comparison on them is a first impression and not a measurement of a distribution.

*Time per property.*
The cactus plots in @fig-cactus-small and @fig-cactus-fam show, for each property, the times of the answered queries sorted from the fastest to the slowest, one curve per tool.
A curve ends early for a tool that answers fewer queries, and the left end of a curve is the cost of the smallest model,
which is dominated by the tool's start-up overhead (@tab-startup).
ITS-Tools high start-up overhead is explained by its JVM-based implementation.
@fig-cactus-large shows the same for the large competition models.

#figure(
  image(export-dir + "figures/e4_cactus_mcc_small.svg", width: 100%),
  caption: [
    Cactus plots for the 90 small MCC models, one panel per property: the answered queries of each tool sorted ascending by time.
    The further a curve extends to the right, the more queries the tool answers, and the lower it runs, the faster.
  ],
) <fig-cactus-small>

#figure(
  image(export-dir + "figures/e4_cactus_fam.svg", width: 100%),
  caption: [
    Cactus plots for the parametrized families, one panel per property, as in @fig-cactus-small.
  ],
) <fig-cactus-fam>

#figure(
  image(export-dir + "figures/e4_cactus_mcc_large.svg", width: 100%),
  caption: [
    Cactus plots for the 20 large competition models, one panel per property, as in @fig-cactus-small.
  ],
) <fig-cactus-large>

*Head to head on the competition models.*
A cactus plot compares the distributions of the query times, but hides how the tools compare on the same model.
@fig-time-small and @fig-time-large therefore plot, for every model and property, the time of `petrivet`
against the time of LoLA and against the time of TINA.

On the small models `petrivet` is at least twice as slow as LoLA on
#time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "lola").n_slower2 of the #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "lola").n_both
deadlock-freedom queries that both answer, and faster by at least a factor 2 on only
#time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "lola").n_faster2.
For liveness and quasi-liveness the models are split: against LoLA, `petrivet` is at least twice as fast on
#time-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "lola").n_faster2 and #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q4" and r.reference == "lola").n_faster2 models,
within a factor 2 on #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "lola").n_equal2 and #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q4" and r.reference == "lola").n_equal2,
and at least twice as slow on #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "lola").n_slower2 and #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q4" and r.reference == "lola").n_slower2,
respectively.

The comparison with TINA is more even: on deadlock-freedom, `petrivet` is at least twice as fast on
#time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "tina").n_faster2 models,
within a factor 2 on #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "tina").n_equal2
and at least twice as slow on #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "tina").n_slower2.
For liveness, quasi-liveness and 1-safety it is within a factor 2 on most models
(#time-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "tina").n_equal2, #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q4" and r.reference == "tina").n_equal2
and #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q5" and r.reference == "tina").n_equal2).

On the models where only one of the two tools answers, the picture depends on the property:
for deadlock-freedom LoLA answers #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "lola").n_only_reference
models that `petrivet` does not, and `petrivet` none that LoLA does not; for liveness `petrivet` answers
#time-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "lola").n_only_petrivet
models that LoLA does not, and LoLA #time-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "lola").n_only_reference
models that `petrivet` does not.
On the large models the two tools both answer at most
#calc.max(..("Q1", "Q3", "Q4", "Q5").map(q => int(time-h2h.find(r => r.source == "mcc-large" and r.query == q and r.reference == "lola").n_both)))
models of a question, so the figure is mostly a picture of where each tool fails:
LoLA answers #time-h2h.find(r => r.source == "mcc-large" and r.query == "Q1" and r.reference == "lola").n_only_reference
deadlock-freedom models that `petrivet` does not, and `petrivet`
#time-h2h.find(r => r.source == "mcc-large" and r.query == "Q1" and r.reference == "lola").n_only_petrivet.

#figure(
  image(export-dir + "figures/e4b_time_scatter_mcc_small.svg", width: 100%),
  caption: [
    Time of `petrivet` (dispatch, x-axis) against the time of LoLA (top row) and TINA (bottom row), one dot per small MCC model and property. \
    Dots above the diagonal are models on which `petrivet` was faster.
  ],
) <fig-time-small>

#figure(
  image(export-dir + "figures/e4b_time_scatter_mcc_large.svg", width: 100%),
  caption: [
    The same as @fig-time-small for the 20 large competition models.
  ],
) <fig-time-large>

*Scaling on the parametrized families.*
While the competition models vary in structure and size at once,
the parametrized families elicit the same structure at differing sizes.
@fig-scaling-time shows, for each family, the time to decide four properties by net size, and
@tab-frontier-all the largest net size at which every property is decided within the time limit.

`petrivet` dispatch decides all four properties up to size 1,024 on
#frontier.filter(r => r.solver_id == "pv-dispatch" and r.max_size_all_solved != "" and float(r.max_size_all_solved) == 1024.0).len()
of the #frontier-families.len() families,
TINA on #frontier.filter(r => r.solver_id == "tina" and r.max_size_all_solved != "" and float(r.max_size_all_solved) == 1024.0).len()
and LoLA on #frontier.filter(r => r.solver_id == "lola" and r.max_size_all_solved != "" and float(r.max_size_all_solved) == 1024.0).len().
On the families whose state spaces grow exponentially with net size
(`sm-hub-and-spokes`, `sm-two-cycles-n-tokens`, `sm-ring-with-sink`, `mg-forkjoin-cycle`, `fc-forkjoin`),
LoLA and TINA time out from size 8 to 16, whereas `petrivet` dispatch reaches size 1,024 (64 on `fc-forkjoin`),
because it decides these properties from the structure of the net alone.
`mg-forkjoin-unbounded` has no size at which the tools decide all four properties.
The picture reverses on `ac-cycle`, `ac-cycle-no-low-token` and `general-cycle`,
whose number of reachable markings grows only polynomially (about $n^2$ for `ac-cycle` and `general-cycle`):
TINA reaches size 1,024 on all three, and `petrivet` dispatch stops at size 64,
where its structural analysis becomes the bottleneck (the memory figure below shows it using 5 to 7 GiB).
LoLA is in between: it reaches size 1,024 only on `circuit` and `sm-two-cycles`,
and times out at size 64 on `ac-cycle` and `general-cycle`.

#figure(
  image(export-dir + "figures/e4b_scaling_time_fam.svg", width: 100%),
  caption: [
    Time taken to decide four properties against the size of the net family, per tool.
    A hollow marker means that at least one run timed out, so the time is a lower bound.
  ],
) <fig-scaling-time>

#figure(
  eval-table(
    (3fr, 1fr, 1fr, 1fr),
    ("Family", solver-name.at("pv-dispatch"), solver-name.at("lola"), solver-name.at("tina")),
    ..frontier-families.map(f => (
      [#family-name.at(f)],
      cell(frontier.find(r => r.family == f and r.solver_id == "pv-dispatch").max_size_all_solved),
      cell(frontier.find(r => r.family == f and r.solver_id == "lola").max_size_all_solved),
      cell(frontier.find(r => r.family == f and r.solver_id == "tina").max_size_all_solved),
    )).flatten(),
    align: (left, right, right, right),
  ),
  caption: [
    The largest size of each family at which each tool decides all four properties within the limit (a dash: no such size).
    The sizes tried are the powers of two from 2 to 1,024.
  ],
) <tab-frontier-all>

*Memory.*
Every tool needs about #dec(float(startup.find(r => r.solver_id == "lola").baseline_rss_kib) / 1024, d: 1) MiB
for a trivial net, except ITS-Tools, which needs #dec(float(startup.find(r => r.solver_id == "its-tools").baseline_rss_kib) / 1024, d: 0) MiB
because of the Java Virtual Machine (see @tab-startup).
@fig-memory-small and @fig-memory-large plot the peak memory of `petrivet` against that of LoLA and TINA for each
MCC model and property, in the same way as @fig-time-small and @fig-time-large for the time.
`petrivet` dispatch uses at least twice as much memory as LoLA on
#mem-h2h.find(r => r.source == "mcc-small" and r.query == "Q1" and r.reference == "lola").n_petrivet_more2
of the 90 small MCC models for deadlock-freedom, and on
#mem-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "lola").n_petrivet_more2
and #mem-h2h.find(r => r.source == "mcc-small" and r.query == "Q4" and r.reference == "lola").n_petrivet_more2
models for liveness and quasi-liveness.
Against TINA the models are split: for liveness `petrivet` uses at least twice as much on
#mem-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "tina").n_petrivet_more2
models and at most half on #mem-h2h.find(r => r.source == "mcc-small" and r.query == "Q3" and r.reference == "tina").n_petrivet_less2.
I did not verify where the additional memory of `petrivet` goes; it includes the fixed cost of the structural
analysis found in the dispatch section.

#figure(
  image(export-dir + "figures/e4b_memory_scatter_mcc_small.svg", width: 100%),
  caption: [
    Peak memory of `petrivet` (dispatch, x-axis) against that of LoLA (top row) and TINA (bottom row) for each small MCC model and property.
    Dots above the diagonal are models on which `petrivet` used less memory.
    A dot with a dark outline includes a run that was ended by the memory limit,
    and a hollow dot includes a run which timed out (its value is the memory when it was stopped, a lower bound).
  ],
) <fig-memory-small>

#figure(
  image(export-dir + "figures/e4b_memory_scatter_mcc_large.svg", width: 100%),
  caption: [The same as @fig-memory-small for the 20 large MCC models.],
) <fig-memory-large>

On the parametric families, @fig-memory-scaling shows the same effect as the time:
on the families with an exponential number of markings LoLA and TINA run into the memory or the time limit with several
GiB in use, whereas `petrivet` dispatch stays at a few tens of MiB.
On the asymmetric-choice and general cycles the order is reversed, and `petrivet` is the tool with the largest memory.

#figure(
  image(export-dir + "figures/e4b_scaling_memory_fam.svg", width: 100%),
  caption: [
    Peak memory against the size of the family, per tool: the largest peak of the four questions at each size (logarithmic axes, MiB; the dashed line is the memory limit of 8 GiB).
    A hollow marker means that some question was stopped by the time limit, so its value is the memory when it was stopped (a lower bound; it can be lower for the largest sizes, because the run got less far in 30 s), and a dark outline that some question was ended by the memory limit.
  ],
) <fig-memory-scaling>

@fig-bytes-per-state shows the memory footprint of the state space of the small MCC models,
for those runs where the tools report the number of markings.

#figure(
  image(export-dir + "figures/e7_bytes_per_state.svg", width: 100%),
  caption: [
    Memory footprint per reachable marking against the number of places,
    for each tool and each MCC competition model on which the tool reports a verdict on the number of markings.
    The start-up memory of the tool is subtracted.
  ],
) <fig-bytes-per-state>

The median bytes per marking is
#dec(float(bytes.find(r => r.solver_id == "lola").median_bytes_per_state), d: 0) for LoLA,
#dec(float(bytes.find(r => r.solver_id == "pv-general").median_bytes_per_state), d: 0) for `petrivet`,
#fmt(float(bytes.find(r => r.solver_id == "tina").median_bytes_per_state)) for TINA and
#fmt(float(bytes.find(r => r.solver_id == "its-tools").median_bytes_per_state)) for ITS-Tools.

A clear positive correlation is visible for `petrivet`, LoLA, and TINA, the explicit-state tools:
the more places, the more memory per marking.
For ITS-Tools, there is seemingly no correlation between the number of places and the memory per marking,
which is explainable by its use of SDDs to store the state space:
depending on how successful the SDD compression is, the memory per marking can be higher or lower than other tools.

It should be noted that `petrivet` is currently unable to generate the set of reachable markings
(the nodes of the reachability graph) without also generating the edges, which is theoretically possible,
and would reduce the memory footprint by a nontrivial factor.

*Start-up.*
@tab-startup shows the time and memory taken by each tool to decide deadlock-freedom for a trivial net.
They are the floor of every measurement above.
The measurement floor of the wrapper is roughly #dec(val("e9.lola.baseline_rss_kib") / 1024, d: 1) MiB,
which unfortunately obscures the true start-up memory of every tool except ITS-Tools.

// TODO: Recheck the measurement floor whether it was subtracted here

#figure(
  eval-table(
    (2fr, 1fr, 1fr),
    ("Tool", "Time (ms)", "Memory (MiB)"),
    ..startup.map(r => (
      [#solver-name.at(r.solver_id)],
      [#dec(float(r.baseline_wall_ms), d: 1)],
      [#dec(float(r.baseline_rss_kib) / 1024, d: 1)],
    )).flatten(),
    align: (left, right, right),
  ),
  caption: [
    Start-up cost of each tool: wall time and peak memory on deadlock-freedom of a trivial net.
  ],
) <tab-startup>

*Limitations.*
- Not every run was repeated, so the measurements are subject to noise.
- The time limit of 30 seconds and the memory limit of 8 GiB are arbitrary and obscure the true performance of the tools
  beyond those limits.
- The memory of a run stopped by the time limit is the memory at that moment and includes the memory footprint of the harness.
- The comparison omits the reachability and coverability properties due to insufficient time.

*Conclusion.*
The comparison shows that `petrivet` is generally comparable with established tools in speed and memory,
with some significant advantages on more restricted classes of nets, and some disadvantages on the less restricted classes.