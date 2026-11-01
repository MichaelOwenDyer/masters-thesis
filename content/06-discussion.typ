#import "./01-introduction/03-research-questions.typ": *

#pagebreak(weak: true)
= Discussion <discussion>

This chapter interprets the results of the evaluation with respect to the research questions and requirements,
summarizes the strengths and weaknesses of `petrivet`,
and reflects on the lessons learned during its implementation.

== Research Questions Revisited

*#rqs.existing-tools: existing tools.*
The survey found no tool that meets all eight requirements (@related-work).
The tools that are strongest in performance and evidence, the contest participants,
are native command-line programs with proprietary input formats and no library interface,
while the tools that are easiest to embed and extend decide few properties and offer no performance evidence.
The gap that motivated `petrivet` is therefore not a missing algorithm, but a missing combination:
the established procedures exist, but not in a form that a process-orchestration system can call, bound, and build on.
`petrivet` was designed for this combination,
and by the same rubric it fully meets six requirements, partly meets two, and fails none (@eval-requirements),
with the caveat that I rated it myself.

*#rqs.correctness: correctness.*
No verdict of any of the three modes of `petrivet` contradicts a reference verdict (@eval-correctness).
This result should be read together with the three bugs found on the way to it.
The most serious of them produced wrong verdicts on reachability and coverability,
and it was only found because the evaluation compared every verdict with two independent tools.
Unit tests did not find it, because they encoded the same misunderstanding as the code.
The evaluation was therefore not only a measurement of correctness but a necessary part of achieving it,
and the zero disagreements hold for the version after these fixes.
The price of soundness is the share of queries answered with "unknown":
on the small MCC models the automatic dispatch gives no verdict for a large share of runs,
mostly because it defensively builds the coverability graph where the reachability graph would have sufficed.
That is the intended trade-off, but the trade-off is currently more conservative than it needs to be (@impl-closing-remarks).

*#rqs.procedure-dispatch: procedure dispatch.*
Dispatching to the cheapest applicable procedure pays off where the theory promises it:
for circuits, state machines and marked graphs, the automatic dispatch is faster than brute force,
and for families with exponentially growing state spaces it answers in milliseconds where brute force times out (@eval-dispatch).
For extended free-choice nets the benefit is marginal, and for asymmetric-choice and general nets the dispatch is slower,
because it pays for the Commoner/Hack criterion and the coverability graph without gaining a polynomial procedure in return.
Two consequences follow.
First, the benefit of dispatch depends on the distribution of structural classes in the models it is applied to.
Most of the competition models are general nets, on which dispatch has little to offer;
whether process models in practice fall into the restricted classes more often is a question this evaluation does not answer.
Second, dispatch is not free: its preliminary checks cost time on exactly the nets where they rarely succeed.
A cheaper order of checks for general nets, or a polynomial procedure for free-choice nets such as the rank theorem,
would address the two classes where dispatch currently loses.
The evaluation also improved the library directly:
investigating the poor first results on the MCC models led to the optimization of $omega$-acceleration,
without which the coverability graph was orders of magnitude slower than the reachability graph on bounded nets.

*#rqs.cost-control: cost control.*
Both mechanisms work as intended for the typical query, with known limits (@eval-cost-control).
The restriction to polynomial procedures answers queries on the parametrized families and the large MCC models
where brute force times out, but rarely gives a verdict on the small MCC models,
for the same reason that dispatch rarely helps there:
few of them belong to a class with a polynomial procedure.
The deadline is kept within a few milliseconds in the median,
but the distribution has a tail caused by stages that do not check the deadline.
For the chair's use case this means that a caller can rely on the deadline in practice,
but not yet as a hard guarantee;
the causes are named, and closing them is a matter of passing the deadline into the remaining stages.

*#rqs.comparison: comparison with established tools.*
`petrivet` is competitive with LoLA and TINA on liveness and on the parametrized families,
and behind them on deadlock-freedom and quasi-liveness on the small MCC models (@eval-comparison).
Given that LoLA and TINA have been developed for decades and optimized for exactly these benchmarks,
being in the same range on some properties is a positive result,
but `petrivet` is not a replacement for a dedicated model checker on hard instances.
The gap on deadlock-freedom is explained by the fallback procedures:
`petrivet` attempts the Commoner/Hack criterion first and then explores the full state space,
whereas the established tools use reductions such as stubborn sets and structural reductions,
which are known but not yet implemented (@impl-closing-remarks).
`petrivet`'s low start-up overhead compared with JVM-based tools matters for the intended use,
in which many small queries are issued from within another program.

== Strengths and Weaknesses

The strengths of `petrivet` lie in the combination that the survey found missing:
a memory-safe library with a typed API, cost control on every query,
evidence with its answers, and verdicts that agree with the reference tools.
Its procedure dispatch makes restricted classes of nets cheap to analyze,
and its procedures are documented with their theoretical foundation.

Its weaknesses are, first, performance on general nets,
where it falls back to state space exploration without the reductions that make the established tools fast;
second, its restriction to ordinary, connected nets, which excluded a third of the MCC models drawn for the evaluation;
third, the uneven evidence returned by its API, where some properties return only a verdict;
and fourth, its development by a single author, which shows in the partial rating for approachability.

== Implementation Discussion

*Separating the API from the core.*
Early versions of `petrivet` made no distinction between the user-facing API and the internal core logic:
the user interacted directly with internal data structures and called internal procedures.
This was simple, but the data the analysis algorithms needed was not always in the form the user wanted to see,
and the performance of an analysis depended on the exact sequence of operations the user performed to construct the net.
The current design separates the two:
the API presents a friendly builder interface which prevents common mistakes at compile time,
while the core works on a dense, canonical index-based representation.
The trade-off is the need for an intermediate translation layer,
which adds design complexity and marginal overhead to every query,
and care must be taken not to let this translation leak into the hot path of the core logic.
The rule that has slowly emerged, though not yet throughout the codebase,
is that the core never depends on the API, and the API only calls into the core and maps its results back.
It has proven a successful way to contain the translation, at the cost of some boilerplate.

*Sharing code between procedures.*
Several procedures that the literature presents separately turned out to share most of their implementation.
The reachability and coverability graphs are explored by the same algorithm,
generic over the marking type, and differ only in the $omega$-acceleration step (@algorithm-state-space-exploration).
The read arc and causality constraints of @thierry-mieg_structural_2020 became a single refinement,
and the maximal siphon and trap computations are one algorithm and its dual.
Sharing paid off in correctness as much as in code size:
a fix to the explorer fixes both graphs, and the reachability graph explorer
doubles as the reference against which the subclass procedures are tested.

*Semi-decision procedures and soundness.*
The most serious bug found during the evaluation was not a programming error but a misreading of the literature.
The increment constraints of @wimmel_applying_2011 steer the search for a realizable firing sequence;
they are not facts about the net, but I had treated an unsatisfiable problem that contained them as a proof of unreachability,
and `petrivet` returned nine wrong verdicts (@eval-correctness).
The lesson is that in a semi-decision procedure the distinction between _what is known_ and _what is assumed_
must be maintained explicitly, and is easy to lose.
Recording why a problem is unsatisfiable,
initially added to explain negative answers to the user,
turned out to be what made this bug diagnosable and fixable at all.

*From theorem to code.*
Translating results from the literature into code was the most error-prone part of the work.
Besides the increment constraints, I misunderstood a suggestion to search for traps in the subnet induced by a Parikh vector,
and implemented a weaker variant that searches the whole net (@impl-reachability-coverability);
I initially implemented asymmetric choice with the wrong one of two definitions in common use;
and I more than once misunderstood the conditions under which a theorem applies as broader than they actually are,
for example a sufficient condition for deadlock-freedom that was used as if it were also necessary.
None of these were caught by unit tests, because the tests encoded the same misreading as the implementation.
They were caught by comparing against independent sources:
the reachability graph on small nets, third-party tools on the evaluation corpus, and conversations with domain experts.
Writing this thesis was itself such a check:
formalizing the encodings and writing pseudocode for them exposed two further discrepancies between intent and implementation.

*Cost control must be designed in from the start.*
Deadlines were added to `petrivet` after most procedures existed,
by threading an optional deadline through each of them.
The evaluation shows the result (@eval-cost-control):
the typical query returns close to its deadline,
but the stages that do not check the deadline run to completion and cause the tail of the distribution.
A deadline only bounds the cost of a query if every loop whose running time depends on the size of the input checks it.
This is easy to forget in each individual procedure, and hard to retrofit,
so it belongs in the design of the procedure interface from the start.
The same holds for the distinction between "no" and "unknown":
because every analysis returns an `Option` from the beginning, the type system enforces that every caller handles the unknown case,
and the efficient-only mode could be added later without changing any signatures.

*Dependencies are part of the correctness surface.*
Choosing a dependency means taking on its limits.
The recursive strongly-connected-component algorithm of `petgraph` overflows the stack on large reachability graphs.
Bugs in the early-stage pure-Rust OxiZ solver (not evaluated) directly impact the correctness of `petrivet`.
Z3 itself brings native C++ code into an otherwise memory-safe library (@eval-requirements).
When correctness is a priority,
the choice of dependencies is just as important as the correctness of the code itself.

*Restricting the scope.*
`petrivet` supports only ordinary Petri nets, without arc weights.
This kept the structural theory and its implementation simple,
since many structural results, such as the Commoner/Hack criterion, are stated for ordinary nets.
But the choice is reflected in the data layout of the core:
presets and postsets are stored as adjacency lists of indices, not as maps from indices to weights,
which is why adding arc weights is not a small change,
and it excluded a third of the MCC models drawn for the evaluation (@eval-requirements).
In hindsight, deciding early which model extensions a library will eventually support
determines how much of it has to be rewritten later.
