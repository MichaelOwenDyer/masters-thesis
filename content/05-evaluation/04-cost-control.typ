#import "../01-introduction/03-research-questions.typ": *
#import "../eval-data.typ": *

== Cost Control <eval-cost-control>

#rqs.cost-control asks whether a caller can effectively bound the cost of an analysis.
Two mechanisms are examined.

The first mechanism is the ability to restrict `petrivet` to polynomial procedures only,
and the hypothesis is that there are nets for which these procedures return a verdict within the time limit
while the general RG exploration procedure times out.
The experiment reports the proportion of runs where the polynomial procedures are able to provide a verdict.

The second mechanism is the ability to set a deadline for the analysis,
and the hypothesis is that `petrivet` returns "close" to the requested deadline.
The definition of "close" was fixed as "within 200ms" before the measured runs began, so that the data cannot influence the definition.
The experiment measures how long beyond the requested deadline the library's automatic dispatch actually returns,
for deadlines of 10, 100, and 1000 milliseconds,
in those runs where a verdict was known to take more than 2 seconds to compute.
The distribution of the overshoot is reported, together with the proportion of runs which lie within the 200ms threshold.
The nets which trigger the largest overshoots are reported as well, because they indicate parts of the implementation
which check the deadline too infrequently and could be improved.

#let h3a(src) = find("e3a.csv", source: src)
#let e3b-row(t) = rows("e3b_overshoot.csv").filter(r => r.scope == "all" and float(r.timeout_ms) == t).first()
#let wall-cap(t) = val("e3b.t" + str(t) + ".n_wall_cap")
#assert(("fam", "mcc-small", "mcc-large", "calibration").map(c => int(h3a(c).n_wrong)).sum() == 0, message: "the text says that no verdict of the fast methods is wrong")

*The polynomial-only mode.*
@tab-h3a shows how often the polynomial procedures are able to produce a verdict.
On the parametrized families they give a verdict for #cell(h3a("fam").n_definite) of #cell(h3a("fam").n_runs)
runs (#pct(float(h3a("fam").share_definite), d: 0)) and decline in the rest.
In #h3a("fam").n_poly_answers_general_timeout of the runs the polynomial methods answer where RG exploration times out,
so the hypothesis is supported.
The same happens for #h3a("mcc-large").n_poly_answers_general_timeout runs of the large competition models.
On the small competition models the efficient methods answer only #pct(float(h3a("mcc-small").share_definite), d: 0)
of the runs, and in none of them does RG exploration timeout:
for MCC nets the restriction to efficient methods mostly prevents a verdict from being returned at all.

#figure(
  eval-table(
    (2.4fr, 1fr, 1fr, 1fr, 1fr, 1.8fr),
    ("Collection", "Runs", "Verdict", "No verdict", "Incorrect", "Verdict where RG exploration times out"),
    ..("calibration", "fam", "mcc-small", "mcc-large").map(c => (
      source-name.at(c),
      cell(h3a(c).n_runs),
      cell(h3a(c).n_definite),
      cell(h3a(c).n_unknown),
      cell(h3a(c).n_wrong),
      cell(h3a(c).n_poly_answers_general_timeout),
    )).flatten(),
  ),
  caption: [
    The number of runs where the fast methods
  ],
) <tab-h3a>

*The deadline.*
#cell(e3b-row(10.0).n_cells) runs which were known to be computable and take >2 seconds were repeated with deadlines
of 10, 100 and 1,000 milliseconds.
@tab-deadline and @fig-overshoot show how long after the deadline the library returned.
The typical overshoot is small: the median is #dec(float(e3b-row(10.0).overshoot_median_ms), d: 1),
#dec(float(e3b-row(100.0).overshoot_median_ms), d: 1)
and #dec(float(e3b-row(1000.0).overshoot_median_ms), d: 1) milliseconds,
and #pct(float(e3b-row(10.0).share_within_threshold), d: 1),
#pct(float(e3b-row(100.0).share_within_threshold), d: 1) and
#pct(float(e3b-row(1000.0).share_within_threshold), d: 1) of the runs return within the target threshold of
#val("e3b.threshold_ms") milliseconds.
The overshoot grows with the deadline.
The hypothesis holds for the typical run, but it is not guaranteed: the distribution has a tail.

#figure(
  eval-table(
    (1.3fr, 0.9fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1.1fr, 1fr),
    ("Deadline (ms)", "Runs", "Timed out", "Median Overshoot (ms)", "90th percentile overshoot (ms)", "95th percentile overshoot (ms)", "Maximum overshoot (ms)", "Proportion within threshold", "Answered after deadline"),
    ..(10.0, 100.0, 1000.0).map(t => (
      cell(t),
      cell(e3b-row(t).n_cells),
      cell(wall-cap(int(t))),
      [#dec(float(e3b-row(t).overshoot_median_ms), d: 1)],
      [#dec(float(e3b-row(t).overshoot_p90_ms), d: 1)],
      [#dec(float(e3b-row(t).overshoot_p95_ms), d: 1)],
      [#dec(float(e3b-row(t).overshoot_max_ms), d: 0)],
      [#pct(float(e3b-row(t).share_within_threshold), d: 1)],
      [#pct(float(e3b-row(t).share_definite), d: 1)],
    )).flatten(),
  ),
  caption: [
    Statistics of the overshoot of `petrivet`'s return time beyond the requested deadline,
    for deadlines of 10, 100 and 1,000 milliseconds.
  ],
) <tab-deadline>

#figure(
  image(export-dir + "figures/e3b_overshoot.svg", width: 85%),
  caption: [
    Distribution of the overshoot time between the deadline and return, for deadlines of 10, 100 and 1,000 ms.
    Bars further left mean that `petrivet` stopped closer to its deadline.
  ],
) <fig-overshoot>

Investigation into the runs with the largest overshoot reveals a structural cause:
the dispatch procedure for boundedness checks the deadline between its stages,
but two of the stages can take a long time to complete and do not check the deadline internally.
This is where the largest overshoots come from (@tab-top-overshoot): the boundedness question on the largest
`fc-forkjoin` and `fc-forkjoin-leaky` nets returns after about 9 seconds whatever the deadline,
and then gives the correct verdict, so the deadline was ignored and not enforced.
The liveness and quasi-liveness questions on one MCC model of the RERS2020 family,
and the deadlock question on a ServersAndClients net, overshoot by about half a second to a second and then give no verdict.
Worse than a late return is no return: for #wall-cap(10) runs at every deadline,
the 1-safety question on one large MCC model (FamilyReunion) and on two small ones (ServersAndClients),
`petrivet` did not return at all, and timed out after 30 seconds.
These are blind spots in the implementation which should be fixed.
Thankfully, the fix is straightforward: the deadline needs to be checked more frequently in those procedures.

#figure(
  eval-table(
    (auto, auto, auto, auto, auto, auto),
    ("Collection", "Net or family", "Property", "Deadline (ms)", "Overshoot (ms)", "Verdict"),
    ..rows("e3b_top_overshoot.csv").slice(0, 10).map(r => (
      [#source-name.at(r.source, default: r.source)],
      [#family-name.at(r.group, default: r.group)],
      [#question-name.at(r.query)],
      cell(r.timeout_ms),
      cell(r.overshoot_ms),
      [#if r.verdict == "UNKNOWN" [none] else [#r.verdict]],
    )).flatten(),
    align: (left, left, left, right, right, left),
  ),
  caption: [The ten largest deadline overshoots.],
) <tab-top-overshoot>

*Summary.*
The evidence generally supports the hypotheses of #rqs.cost-control, with some caveats.
The polynomial procedures are able to answer some queries where the general RG exploration times out,
but for many real-world Petri nets they are too limited to answer at all.
The deadline mechanism is effective for the typical run, but there are blind spots in the implementation which
allow `petrivet` to overshoot the deadline by a second or more, and even to not return at all.
This can be fixed by checking the deadline more frequently in the affected procedures.