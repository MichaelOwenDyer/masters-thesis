#import "../01-introduction/03-research-questions.typ": *
#import "../eval-data.typ": *

== Correctness <eval-correctness>

#rqs.correctness asks whether `petrivet` ever produces an incorrect verdict.
The hypothesis is that it does not, on any model, for any property, from any procedure.
"Unknown" is a legitimate response and counts as neither correct nor incorrect.
All three operating modes of `petrivet` (per @eval-tools) are evaluated; disagreement between them is a correctness violation.
The section first reports how many total cases could be compared and how many were excluded,
then presents the agreement rate and the upper bound on the disagreement rate.

#let collections = ("gen", "gen-lola-only", "fam", "mcc-small", "mcc-large", "calibration")
#let modes = ("pv-dispatch", "pv-general", "pv-poly")
#let agreement(m, c) = find("e1_by_solver_source.csv", solver_id: m, source: c)
#let compared(c) = modes.map(m => int(agreement(m, c).n_conclusive)).sum()
#let disagreeing(c) = modes.map(m => int(agreement(m, c).n_disagree)).sum()
#let total-compared = collections.map(compared).sum()
#let total-disagreeing = collections.map(disagreeing).sum()
#assert(total-disagreeing == 0, message: "the text says that no verdict of the three modes contradicts a reference verdict")
#let auto-ub(c) = float(agreement("pv-dispatch", c).cp_upper_95)
#let auto-cells(c) = int(agreement("pv-dispatch", c).n_conclusive) + int(agreement("pv-dispatch", c).n_inconclusive)

@tab-agreement shows, per net collection, how many verdicts of each of the three operating modes of `petrivet`
could be compared with a reference verdict.
Over all collections and modes, #fmt(total-compared) runs could be compared, and #fmt(total-disagreeing)
of them disagreed with the reference verdict.
The statistical strength of this result depends on the net collection.
For `petrivet`'s automatic dispatch operating mode, the one-sided 95 percent upper bound on the rate of disagreement is
#pct(auto-ub("gen"), d: 3) on the randomized nets (the two-tool group),
#pct(auto-ub("gen-lola-only"), d: 2) on the group confirmed by LoLA alone,
#pct(auto-ub("fam"), d: 2) on the parametrized families and
#pct(auto-ub("mcc-small"), d: 2) on the small competition models.
On the large competition models the automatic mode gave a verdict in only #int(agreement("pv-dispatch", "mcc-large").n_conclusive) of
#auto-cells("mcc-large") runs, so the bound is #pct(auto-ub("mcc-large"), d: 0) and the large models say little about correctness.

#figure(
  eval-table(
    (2.4fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr),
    ("Collection", "Automatic dispatch", "RG exploration", "Efficient methods only", "Disagree-ments", "Upper bound (dispatch)", "No verdict (dispatch)"),
    ..collections.map(c => (
      source-name.at(c),
      cell(agreement("pv-dispatch", c).n_conclusive),
      cell(agreement("pv-general", c).n_conclusive),
      cell(agreement("pv-poly", c).n_conclusive),
      cell(disagreeing(c)),
      [#pct(auto-ub(c), d: 2)],
      [#pct(float(agreement("pv-dispatch", c).inconclusive_rate), d: 1)],
    )).flatten(),
  ),
  caption: [Verdicts of the three modes of `petrivet` that could be compared with a reference verdict, per collection, the disagreements,
    the one-sided 95 percent upper bound on the disagreement rate of the automatic mode, and the share of runs without a verdict.],
) <tab-agreement>

For the automatic dispatch mode with the small MCC models, #fmt(int(find("e4.csv", source: "mcc-small", query: "Q0", solver_id: "pv-dispatch").n_cells) + int(find("e4.csv", source: "mcc-small", query: "Q6", solver_id: "pv-dispatch").n_cells))
of the runs are the two properties for which it has no procedure other than plain exploration
(state space size and stable place), and most of the rest are time-outs or memory-outs due to the library's
defensive construction of the coverability graph instead of the faster to construct,
but potentially infinite, reachability graph.

For reachability and coverability, which are checked on the randomized nets only, @tab-qrqc separates the cases by the verdict and by the kind of reference.
The group confirmed by LoLA alone is skewed: for coverability, #fmt(int(find("e5a.csv", solver_id: "pv-dispatch", query: "QC", tier: "lola_only", stratum: "yes").n_conclusive))
of its verdicts are positive and only #fmt(int(find("e5a.csv", solver_id: "pv-dispatch", query: "QC", tier: "lola_only", stratum: "no").n_conclusive)) negative,
because a reachable or coverable target of an unbounded net is much easier to find than an unreachable one is to refute.
No verdict of either group contradicts the reference.

#let qrqc = (("QR", "two_tool"), ("QR", "lola_only"), ("QC", "two_tool"), ("QC", "lola_only"))
#figure(
  eval-table(
    (1.2fr, 1.9fr, 1.3fr, 1fr, 1fr, 1fr, 1fr),
    ("Property", "Reference", "Verdict", "Compared", "Disagree-ments", "Upper bound", "No verdict"),
    align: (left, left, left, right, right, right, right),
    ..qrqc.map(((q, t)) => ("yes", "no").map(st => {
      let r = find("e5a.csv", solver_id: "pv-dispatch", query: q, tier: t, stratum: st)
      (
        question-name.at(q),
        if t == "two_tool" [LoLA and TINA agree] else [LoLA alone],
        if st == "yes" { if q == "QR" [reachable] else [coverable] } else { if q == "QR" [not reachable] else [not coverable] },
        cell(r.n_conclusive),
        cell(r.n_disagree),
        [#pct(float(r.cp_upper_95), d: 2)],
        cell(r.n_inconclusive),
      )
    })).flatten().flatten(),
  ),
  caption: [
    Reachability and coverability on the randomized nets: verdicts of `petrivet` dispatch against the reference.
    "No verdict" counts the runs with a reference verdict for which `petrivet` returned no verdict.
  ],
) <tab-qrqc>

#let its-sources = ("fam", "mcc-small", "mcc-large", "calibration")
#assert(its-sources.map(c => int(agreement("its-tools", c).n_disagree)).sum() == 0, message: "the text says that no ITS-Tools answer contradicts a reference answer")
#assert(val("e8.unbounded_q1_answered") == val("e8.unbounded_q1_contradict_truth"), message: "the text says that every ITS-Tools answer on an unbounded net contradicts the truth")

ITS-Tools, which was not used as a reference, was checked against the same reference verdicts.
In the collections where it ran, #fmt(its-sources.map(c => int(agreement("its-tools", c).n_conclusive)).sum()) of its verdicts could be compared, and
#fmt(its-sources.map(c => int(agreement("its-tools", c).n_disagree)).sum()) of them contradict the reference.
On nets that are unbounded, which are outside the scope of the comparison, it decided deadlock-freedom for #val("e8.unbounded_q1_answered") nets,
and every one of these verdicts contradicts the construction truth:
it reports a reachable deadlock where there is none.
That is an observation about the tool, not about the comparison.

All three `petrivet` modes were also checked for cache-coherence.
`petrivet` caches intermediate results about deadlock-freedom, boundedness, and liveness properties,
which represents a potential source of error:
if the cache is not coherent, the verdict to one property may vary depending on which properties were decided before.
On #val("e1b.n_nets") randomized nets, the properties were decided in several orders, and every verdict was compared with the
verdict to the same property on a fresh `PetriNet` and, where the reachability graph is small, with an explicit-state
computation. There were #val("e1b.n_differences") differences (upper bound #pct(val("e1b.cp_upper_95"), d: 1)).

*What the evaluation found.*
Three bugs were discovered and fixed during the correctness evaluation, which is a good sign that the evaluation is meaningful.

In the first runs of the randomized nets, the automatic dispatch mode of `petrivet` answered nine reachability and coverability
queries incorrectly: it falsely proved targets unreachable/uncoverable where they were in fact reachable/coverable.
Eight of the nine runs had a reference verdict (five confirmed by both tools, three by LoLA alone) and one had none;
all but one net belonged to the general class.
The cause was a misinterpretation of "increment constraints" as described by @wimmel_applying_2011:
added to the SMT problem to steer the search for a realizable firing sequence, they are not a fact about the net but
a navigation step that can be helpful in finding a solution.
An SMT solver may return UNSAT in the presence of an increment constraint even if the target is reachable/coverable,
but the library trusted that as a definite refutation.
The library now reports "unknown" for such a refutation; the 5,315 negative verdicts of the automatic dispatch mode
for these properties were removed and run again, and no disagreement remains:
the nine problematic runs are now "unknown", and the other 5,306 runs are unchanged.

The second discovered bug was a stack overflow in a dependency during reachability graph analysis.
`petrivet` computes the strongly connected components of the reachability graph for liveness, using Tarjan's algorithm
implemented in the `petgraph` crate #footnote[https://docs.rs/petgraph/0.8.3/petgraph/algo/scc/tarjan_scc/index.html].
This implementation is recursive, and overflows the stack on reachability graphs of roughly $10^5$ states or more.
Switching to an alternative iterative algorithm (Kosaraju's #footnote[https://docs.rs/petgraph/0.8.3/petgraph/algo/scc/kosaraju_scc/index.html]) fixed the problem.

The third bug was a case in which the evaluation harness itself misled the evaluation:
the adapter of one reference tool read output that had been truncated due to excessive length,
and interpreted an intermediate message as the final verdict.
Two disagreements with the reference verdicts revealed this defect;
the adapter was corrected and the affected runs were repeated.

None of these three bugs is visible in the final data.
They are listed here solely for transparency and to illustrate the value of the evaluation.
