#import "../01-introduction/03-research-questions.typ": *
#import "../eval-data.typ": *

== Benefit of Procedure Dispatch <eval-dispatch>

#rqs.procedure-dispatch asks whether `petrivet`'s internal dispatch of procedures, instead of always using one
general procedure, reduces the cost of analysis. The hypothesis is that, for nets of restricted structural class,
`petrivet`'s automatic choice is cheaper than the RG exploration of the state space,
with a positive paired effect after correction for the number of structural classes tested.
The experiment uses the parametrized families of nets, which are constructed to be of a specific structural class
and have a high complexity ceiling. The randomized nets are not used because they are too small for reliable timings.
The MCC models are used, but only the cases where dispatch can select a specialized procedure over the general approach.
Two of the nine queries (state space size and stable place) are only implemented in `petrivet` via state space exploration,
and are therefore excluded from this experiment.

#let classes = ("circuit", "sm", "mg", "efc", "ac", "general")
#let h2(c) = find("e2_paired_tests.csv", cls: c)
#let supported(c) = h2(c).reject_h2 == "true"
#let supported-without-stopped(c) = h2(c).sens_reject_h2 == "true"
#assert(supported("efc") and not supported-without-stopped("efc"), message: "the text says that extended free-choice nets are supported only with the stopped runs")
#assert(not supported("ac") and not supported("general"), message: "the text says that asymmetric-choice and general nets are not supported")

*Method.*
Every query is run twice: once with `petrivet`'s automatic dispatch and once with RG exploration.
The time taken by the two runs is compared.
The "performance ratio" is the time of the RG exploration divided by the time of the automatic dispatch:
greater than 1 means that the automatic dispatch is faster, and less than 1 means that it is slower.
The performance ratio of a structural net class depends on which net sizes happened to be tried and on the time limit,
so a single number per class would be arbitrary: a larger ceiling for the net sizes would produce larger ratios wherever brute force gets stuck.
For that reason the performance ratio is reported per family and per size, and the per-class comparison only asks whether the automatic dispatch is faster or slower.
Runs which time out at 30 seconds are counted as 30 seconds, which is an underestimation of the true time.
The comparisons are therefore additionally repeated excluding the runs that timed out in either mode.

*Results on the parametrized families.*
@tab-h2 shows the results of the paired runs for each structural class.
The automatic dispatch is clearly faster than RG exploration for
#classes.filter(c => supported(c) and supported-without-stopped(c)).map(c => class-name.at(c)).join(", ", last: " and ")
nets, and this holds with and without the timed out runs.
For extended free-choice nets, the automatic dispatch is faster only when the timed out runs are counted,
and RG exploration is generally faster on the queries that both modes finish.
For asymmetric-choice and general nets the automatic dispatch is not faster.

#figure(
  eval-table(
    (1.6fr, 0.8fr, 1fr, 1fr, 0.9fr, 1.2fr),
    ("Class", "Paired Runs", "One or both runs timed out", "p-value", "Faster", "Faster without timeouts"),
    ..classes.map(c => (
      class-name.at(c),
      cell(h2(c).n_pairs),
      cell(h2(c).n_censored),
      p-value(h2(c).holm_p),
      if supported(c) [yes] else [no],
      if supported-without-stopped(c) [yes] else [no],
    )).flatten(),
  ),
  caption: [Performance of automatic dispatch against reachability graph exploration on the parametrized families, by structural class.],
) <tab-h2>

*Speedup by size.*
@fig-performance-ratios shows the median performance ratios of every family at every size that was tried.
A filled-in circle means that every run at that net size finished.
an outlined upward triangle means that RG exploration timed out (the real ratio is higher than shown),
an outlined downward triangle means that the automatic dispatch timed out (the real ratio is lower than shown),
and an outlined diamond means that at least one run of both modes timed out (the real ratio is unknown).
Paired runs in which both runs timed out say nothing about the difference and are left out of the comparison.
Four patterns show:
- For `circuit`, `sm-choice-ring`, `sm-two-cycles`, `mg-buffer-ring` and `mg-buffer-ring-unmarked-circuit`,
  the performance ratio is constant and modest, between about 1 and 2 at every size. The dispatch helps marginally,
  but the number of reachable markings is small enough that RG exploration is not a problem.
- For families with exponentially growing state spaces (`sm-hub-and-spokes`, `sm-ring-with-sink`, `sm-two-cycles-n-tokens`,
  and `mg-forkjoin-cycle`), the performance ratio grows significantly with the size of the net, and from size 16 or 32,
  RG exploration times out on most or all questions while the automatic dispatch still answers in milliseconds.
  The ratio is at least several hundred.
  `mg-forkjoin-unbounded` is a special case: RG exploration hopelessly enumerates an infinite state space,
  and never reaches a conclusion.
- For extended free-choice nets, the dispatch is about even with RG exploration at small sizes,
  until RG exploration begins to time out, and then the dispatch is faster until it also begins to time out.
- For asymmetric-choice and general nets, the dispatch is slower than brute force as the nets grow,
  down to a ratio of about #dec(float(find("e2_speedup_by_size.csv", family: "ac_cycle", size: "64").median_speedup), d: 3)
  for `ac-cycle` at size 64 (the automatic dispatch takes about
  #fmt(1 / float(find("e2_speedup_by_size.csv", family: "ac_cycle", size: "64").median_speedup)) times as long), and
  #dec(float(find("e2_speedup_by_size.csv", family: "general_cycle", size: "64").median_speedup), d: 2) for `general-cycle`.
  The slowdown is consistent with the nontrivial cost of the Commoner-Hack criterion and the Karp-Miller coverability graph,
  which are used by the automatic dispatch on these nets.
  Only at size 1,024 does the automatic dispatch return a verdict where brute force times out.

#figure(
  image(export-dir + "figures/e2_speedup_by_size.svg", width: 100%),
  caption: [
    Median performance ratios per family as the net grows.
    A filled-in symbol means that both runs finished,
    an outlined upward triangle means that RG exploration timed out,
    an outlined downward triangle means that the automatic dispatch timed out,
    and an outlined diamond means that at least one run of both modes timed out.
  ],
) <fig-performance-ratios>

@fig-scaling shows how the times of each mode grow with the size of the net,
and @fig-scatter illustrates every pair of runs individually, colored by family.

#figure(
  image(export-dir + "figures/e2_scaling.svg", width: 100%),
  caption: [Response times by family as the net grows: the automatic dispatch (blue) and RG exploration (orange)],
) <fig-scaling>

#figure(
  image(export-dir + "figures/e2_paired_scatter.svg", width: 85%),
  caption: [Every dot is a pair of runs: the time of the automatic dispatch (x-axis) against the time of RG exploration (y-axis), colored by family.],
) <fig-scatter>

#let frontier-raw(f, s) = find("e2_frontier.csv", family: f, solver_id: s).max_size_solved
#let frontier(f, s) = if frontier-raw(f, s) == "" { 0.0 } else { float(frontier-raw(f, s)) }
#let families = rows("e2_frontier.csv").map(r => r.family).dedup()
#let largest-tried = calc.max(..rows("e2_frontier.csv").map(r => float(r.max_size_attempted)))
#let larger = families.filter(f => frontier(f, "pv-dispatch") > frontier(f, "pv-general"))
#let smaller = families.filter(f => frontier(f, "pv-dispatch") < frontier(f, "pv-general"))

*How large a net can be handled.*
@tab-frontier shows the largest size ($n$) of each family that a mode answers every query for within 30 seconds.
The automatic dispatch handles larger nets than RG exploration in #larger.len() of the #families.len() families,
and RG exploration handles larger nets in #smaller.len() families.
In the remaining families both modes reach the same size, and in
#families.filter(f => frontier(f, "pv-dispatch") == frontier(f, "pv-general") and frontier(f, "pv-dispatch") == largest-tried).len()
of those it is the largest size that was tried, #fmt(largest-tried).
Note that `mg-forkjoin-unbounded` is unbounded, so the RG exploration was unable to finish at any size,
while the automatic dispatch answered all queries at size 1,024.

#figure(
  eval-table(
    (3fr, 1fr, 1fr, 1fr, 1fr),
    ("Family", "Largest size without timeouts for automatic dispatch", "Largest size without timeouts for RG exploration", "Largest size without a timeout in either mode", "Speedup at that size"),
    ..families.map(f => (
      [#family-name.at(f)],
      cell(frontier-raw(f, "pv-dispatch")),
      cell(frontier-raw(f, "pv-general")),
      cell(find("e2_family_summary.csv", family: f).last_clean_size),
      cell(find("e2_family_summary.csv", family: f).last_clean_speedup, d: 2),
    )).flatten(),
    align: (left, right, right, right, right),
  ),
  caption: [
    The largest size parameters that each mode answers every query for without timing out,
    and the performance ratio of the two modes at the highest size where neither mode times out.
    The sizes tried are the powers of two from 2 to 1,024. A dash means that there is no such size.
  ],
) <tab-frontier>

*Memory usage.*
@fig-memory shows the peak memory of both modes for every family and size.

#figure(
  image(export-dir + "figures/e2_memory_by_size.svg", width: 100%),
  caption: [Peak memory of the automatic choice (blue) and plain exploration (orange) per family and size, as the median over the questions that have a memory value (logarithmic axes).
    The dashed line is the memory limit of 8 GiB; circled points are medians in which the run was ended by the memory limit, so their value is the limit and not the memory the run would have needed.],
) <fig-memory>

*The competition models.*
Unlike the parametrized families, the MCC models are not designed to be of a specific structural class,
and dispatch is less often able to select a specialized procedure over the general approach.
Of the #val("e2.mcc_small.n_instances") small models, only #val("e2.mcc_small.n_non_general_class") belong to a
non-general class (#find("e2_mcc_small_classes.csv", cls: "ac").n_instances asymmetric-choice,
#find("e2_mcc_small_classes.csv", cls: "efc").n_instances extended free-choice,
#find("e2_mcc_small_classes.csv", cls: "mg").n_instances marked graph
and #find("e2_mcc_small_classes.csv", cls: "sm").n_instances state machine),
and the other #find("e2_mcc_small_classes.csv", cls: "general").n_instances are general nets.
For #val("e2.mcc_small.n_dispatch_differs") of the #val("e2.mcc_small.n_instances") models
(#pct(val("e2.mcc_small.share_dispatch_differs"), d: 0)) the automatic choice can do something other than explore,
either because the net belongs to a restricted class or because the efficient methods alone answer at least one query
(#val("e2.mcc_small.n_poly_definite") models).
For all other models it can only explore, and any difference to the general mode is then pure overhead of the
Karp-Miller coverability graph vs. the reachability graph.

The first measurement on these models was bad for the automatic dispatch.
On liveness it answered only
#find("e2b_dispatch_mcc.csv", state: "before", source: "mcc-small", query: "Q3").n_dispatch_answered of #find("e2b_dispatch_mcc.csv", state: "before", source: "mcc-small", query: "Q3").n_cells
models, against #find("e2b_dispatch_mcc.csv", state: "before", source: "mcc-small", query: "Q3").n_general_answered for
RG exploration, and where both answered, it took a median #dec(float(find("e2b_dispatch_mcc.csv", state: "before", source: "mcc-small", query: "Q3").ratio_median), d: 1) times as long.
I timed the individual stages of the library on single models to find out why, and found two causes.
These timings were taken by hand and are not part of the figures and tables.

The first cause was in the implementation of the Karp-Miller coverability graph,
which requires updating token counts of every discovered marking to $omega$
where there exists a predecessor marking that it covers (so-called "omega acceleration").
This necessarily brings increasing overhead as the graph grows in size,
but the implementation was not optimized for the bounded case:
it always checked all predecessors, even those which were added after the marking was first discovered.
On a bounded net with 16,587 reachable states this took 51 seconds, whereas constructing the reachability graph took just
12 milliseconds.
This was addressed by initially only comparing each new marking to the markings on the path by which it was first discovered,
and only falling back to checking all predecessors if/when an $omega$ is found.
This produces identical results in my testing, and the construction time dropped to 130 milliseconds on the same net.
I did not prove that the new approach produces identical results in general,
but what it produces is still a sound coverability graph.

#let e2b(state, q, field) = find("e2b_dispatch_mcc.csv", state: state, source: "mcc-small", query: q).at(field)

@fig-dispatch-mcc shows the two modes on every small model, before and after the optimization.
Several runs which automatic dispatch previously timed out on now finish:
after the optimization the automatic choice verdicts
#e2b("after", "Q3", "n_dispatch_answered") of #e2b("after", "Q3", "n_cells") liveness cells
(before: #e2b("before", "Q3", "n_dispatch_answered")), and on the models that both modes answer it is at least twice as
slow on #e2b("after", "Q3", "n_slower2"), about equal on #e2b("after", "Q3", "n_equal2") and at least twice as fast on
#e2b("after", "Q3", "n_faster2").
Deadlock-freedom queries are nearly unaffected, presumably because of the second cause, explained next.

#figure(
  image(export-dir + "figures/e2b_scatter.svg", width: 100%),
  caption: [The automatic choice against RG exploration on the 90 small competition models, one panel per query, before and after the optimization of omega-acceleration described above.],
) <fig-dispatch-mcc>

The second cause remains in the codebase at the time of writing:
for free-choice and asymmetric-choice nets, the automatic dispatch attempts to use the Commoner-Hack criterion to decide
the property where applicable (free-choice liveness, deadlock-freedom), but this does not pay off for the majority of Petri nets.
On one asymmetric-choice net with only 204 reachable states, the Commoner-Hack criterion took 32 seconds to return,
while the reachability graph took 1.7 milliseconds.
Further investigation is needed to determine when the Commoner-Hack criterion is worth using, and the dispatch should be
adjusted accordingly in a future version of the library.

*Summary.*
Dispatching to specialized procedures by the structure of the net pays off where the structure allows a shortcut,
and where the number of states explodes.
On unstructured benchmark nets it costs time, because structural checks and the conservative coverability-graph fallback
add overhead to what is otherwise the same exploration.