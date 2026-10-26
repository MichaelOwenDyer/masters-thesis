#import "../01-introduction/03-research-questions.typ": *
#import "../eval-data.typ": *

== Benefit of Procedure Dispatch <eval-dispatch>

#rqs.procedure-dispatch asks whether `petrivet`'s internal dispatch of procedures, instead of always using one
general procedure, reduces the cost of analysis. The hypothesis is that, for nets of restricted structural class,
`petrivet`'s automatic choice is cheaper than the brute-force exploration of the state space,
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
Every query is run twice: once with `petrivet`'s automatic dispatch and once with brute-force exploration.
The time taken by the two runs is compared.
This is called a paired test.
The "performance ratio" is the time of the brute-force exploration divided by the time of the automatic dispatch:
greater than 1 means that the automatic dispatch is faster, and less than 1 means that it is slower.
The performance ratio of a structural net class depends on which net sizes happened to be tried and on the time limit,
so a single number per class would be arbitrary: a larger ceiling for the net sizes would produce larger ratios wherever brute force gets stuck.
For that reason the performance ratio is reported per family and per size, and the per-class comparison only asks whether the automatic dispatch is faster or slower.
Runs which time out at 30 seconds are counted as 30 seconds, which is an underestimation of the true time.
The comparisons are therefore additionally repeated excluding the paired tests that timed out in either mode.

*Results on the parametrized families.*
@tab-h2 shows the results of the paired tests for each structural class.
The automatic dispatch is clearly faster than brute-force exploration for
#classes.filter(c => supported(c) and supported-without-stopped(c)).map(c => class-name.at(c)).join(", ", last: " and ")
nets, and this holds with and without the timed out runs.
For extended free-choice nets, the automatic dispatch is faster only when the timed out runs are counted,
and brute-force exploration is generally faster on the tests that both modes finish.
For asymmetric-choice and general nets the automatic dispatch is not faster:
the preemptive Commoner-Hack check on asymmetric choice nets does not pay off,
and the dispatch's fallback to the Karp-Miller coverability graph is slower than the brute-force exploration
of the state space for bounded nets (the trade-off is that it terminates for unbounded nets).

#figure(
  eval-table(
    (1.6fr, 0.8fr, 1fr, 1fr, 0.9fr, 1.2fr),
    ("Class", "Paired Tests", "One or both runs timed out", "p-value", "Faster", "Faster without timeouts"),
    ..classes.map(c => (
      class-name.at(c),
      cell(h2(c).n_pairs),
      cell(h2(c).n_censored),
      p-value(h2(c).holm_p),
      if supported(c) [yes] else [no],
      if supported-without-stopped(c) [yes] else [no],
    )).flatten(),
  ),
  caption: [Performance of automatic dispatch against brute-force exploration on the parametrized families, by structural class.],
) <tab-h2>

*Speedup by size.*
@fig-performance-ratios shows the median performance ratios of every family at every size that was tried.
A filled-in circle means that every run at that net size finished.
an outlined upward triangle means that brute-force exploration timed out (the real ratio is higher than shown),
an outlined downward triangle means that the automatic dispatch timed out (the real ratio is lower than shown),
and an outlined diamond means that at least one run of both modes timed out (the real ratio is unknown).
Paired tests in which both runs timed out say nothing about the difference and are left out of the comparison.
Four patterns show:
- For the plain ring, the state machines with a choice ring or two cycles with one token, and the buffer rings,
  the performance ratio is constant and modest, between about 1 and 2 at every size. The dispatch helps marginally,
  but the number of reachable markings is small enough that brute-force exploration is not a problem.
- For families with exponentially growing state spaces (hub and spokes, ring with a token sink, two cycles with $n$ tokens,
  and the cyclic fork and join), the performance ratio grows significantly with the size of the net, and from size 16 or 32,
  brute-force exploration times out on most or all questions while the automatic dispatch still answers in milliseconds.
  The ratio is at least several hundred.
  The unbounded fork and join is a special case: brute-force exploration hopelessly enumerates an infinite state space,
  and never reaches a conclusion.
- For extended free-choice nets, the dispatch is about even with brute-force exploration at small sizes,
  until brute-force exploration begins to time out, and then the dispatch is faster until it also begins to time out.
- For asymmetric-choice and general nets, the dispatch is slower than brute force as the nets grow,
  down to a ratio of about #dec(float(find("e2_speedup_by_size.csv", family: "ac_cycle", size: "64").median_speedup), d: 3)
  for `ac_cycle` at size 64 (the automatic dispatch takes about
  #fmt(1 / float(find("e2_speedup_by_size.csv", family: "ac_cycle", size: "64").median_speedup)) times as long), and
  #dec(float(find("e2_speedup_by_size.csv", family: "general_cycle", size: "64").median_speedup), d: 2) for `general_cycle`.
  The slowdown is consistent with the nontrivial cost of the Commoner-Hack Criterion and the Karp-Miller coverability graph,
  which are used by the automatic dispatch on these nets.
  Only at size 1,024 does the automatic dispatch answer where brute force is stopped.

#figure(
  image(export-dir + "figures/e2_speedup_by_size.svg", width: 100%),
  caption: [
    Median performance ratios per family as the net grows.
    A filled-in symbol means that both runs finished,
    an outlined upward triangle means that brute-force exploration timed out,
    an outlined downward triangle means that the automatic dispatch timed out,
    and an outlined diamond means that at least one run of both modes timed out.
  ],
) <fig-performance-ratios>

@fig-scaling shows how the times of each mode grow with the size of the net,
and @fig-scatter illustrates every paired test individually, colored by family.

#figure(
  image(export-dir + "figures/e2_scaling.svg", width: 100%),
  caption: [Response times by family as the net grows: the automatic dispatch (blue) and brute-force exploration (orange)],
) <fig-scaling>

#figure(
  image(export-dir + "figures/e2_paired_scatter.svg", width: 85%),
  caption: [Each dot is one paired test: the time of the automatic dispatch (x-axis) against the time of brute-force exploration (y-axis), colored by family.],
) <fig-scatter>

#let frontier-raw(f, s) = find("e2_frontier.csv", family: f, solver_id: s).max_size_solved
#let frontier(f, s) = if frontier-raw(f, s) == "" { 0.0 } else { float(frontier-raw(f, s)) }
#let families = rows("e2_frontier.csv").map(r => r.family).dedup()
#let largest-tried = calc.max(..rows("e2_frontier.csv").map(r => float(r.max_size_attempted)))
#let larger = families.filter(f => frontier(f, "pv-dispatch") > frontier(f, "pv-general"))
#let smaller = families.filter(f => frontier(f, "pv-dispatch") < frontier(f, "pv-general"))
*How large a net can be handled.*
@tab-frontier shows the largest size of each family that a mode answers within the 30 seconds.
The automatic dispatch handles larger nets than brute-force exploration in #larger.len() of the #families.len() families and smaller nets in #smaller.len().
In the remaining families both modes reach the same size, and in #families.filter(f => frontier(f, "pv-dispatch") == frontier(f, "pv-general") and frontier(f, "pv-dispatch") == largest-tried).len() of those it is the largest size that was tried, #fmt(largest-tried).

#figure(
  eval-table(
    (3fr, 1fr, 1fr, 1fr, 1fr),
    ("Family", "Automatic dispatch", "Brute-force exploration", "Largest size without a stopped run", "Speedup there"),
    ..families.map(f => (
      [#family-name.at(f)],
      cell(frontier(f, "pv-dispatch")),
      cell(frontier(f, "pv-general")),
      cell(find("e2_family_summary.csv", family: f).last_clean_size),
      cell(find("e2_family_summary.csv", family: f).last_clean_speedup, d: 2),
    )).flatten(),
    align: (left, right, right, right, right),
  ),
  caption: [
    The largest size parameters that each mode answers without timing out, and the largest size at which no run of the family was stopped, with the median ratio of the automatic dispatch at that size (a ratio measured without any stopped run).
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