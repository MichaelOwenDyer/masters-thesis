= Related Work <related-work>

This chapter explores to what extent existing Petri net analysis tools meet the requirements outlined in RQ1:
programmatic access (R1),
safe portability (R2),
cost control (R3),
model interchange (R4),
performance and memory efficiency (R5),
result traceability (R6),
extensibility (R7),
and contribution-friendliness (R8).
I first describe the methodology used to select the tools to survey, and then present the 37 selected tools
with regard to their support for the requirements.
Finally, I state the conclusions drawn from the survey and state the gap that `petrivet` aims to fill.

== Survey Methodology

I followed a structured literature review protocol @kitchenham_systematic_2009 to identify relevant Petri net analysis tools.
The protocol was fixed before the survey was conducted.
I searched ten sources: // todo: update last accessed dates
Google Scholar #footnote[scholar.google.com, last accessed: 2026-10-14],
DBLP #footnote[dblp.org, last accessed: 2026-10-14],
the ACM Digital Library #footnote[dl.acm.org, last accessed: 2026-10-14],
and SpringerLink #footnote[link.springer.com, last accessed: 2026-10-14] for papers;
crates.io #footnote[crates.io, last accessed: 2026-10-14],
PyPI #footnote[pypi.org, last accessed: 2026-10-14],
and GitHub #footnote[github.com, last accessed: 2026-10-14] for software packages;
the Hamburg Petri Nets Tool Database #footnote[www2.informatik.uni-hamburg.de/tgi/PetriNets/tools/db.html],
and the participant list of the 2025 Model Checking Contest. TODO: add 2026 tools
In addition, the tools already known through prior experience were included in the survey.
A set of search queries was defined for each source, and the top N search results by relevance were collected.

A record entered the review in two stages.
At stage 1, search results were roughly filtered for relevance based on title, description/abstract, and metadata.
At stage 2, the remaining records were filtered for relevance by reading the documentation and/or source code of the tool.
A tool was included if its documentation mentioned a decision procedure for place/transition nets
(or nets which can be interpreted as such) for at least one of boundedness, deadlock-freedom, liveness, reachability
or coverability (Inclusion Criterion 1, IC1);
if it is at least partially documented and available for download (IC2);
and if its last release or commit is dated within ten years of the survey, 2016-10-14 or later (IC3).
Pure editors and simulators without analysis capabilities were excluded (Exclusion Criterion 1, EC1),
as well as tools which only support colored, timed, stochastic, or other non-ordinary Petri nets (EC2).

IC3 was widened from 5 to 10 years after the survey had begun, in order to include LoLA @schmidt_lola_2000, a prominent tool which
was last released in 2020. This and other protocol amendments are documented in the survey log @appendix.

The 785 records collected from the ten sources were filtered in stage 1 with the help of Claude Sonnet 5.5,
and a random sample of 20% (157) of the records was filtered by hand to verify the quality of the filtering.
Agreement between the two filtering methods was 94.3%, and Cohen's kappa was a moderate 0.50, which is explained
by the low proportion of results which contained a relevant tool.
No disagreement was responsible for adding or removing a tool from the survey.
46 records passed stage 1 and were filtered by hand in stage 2, resulting in 37 tools which met the inclusion criteria.

== Survey Results

The 37 tools fall into five categories based on their primary purpose:

*Performance-oriented model checkers* (nine tools) are built with LTL and CTL properties in mind:
TAPAAL/VerifyPN @jensen_tapaal_2016,
TINA,
ITS-Tools,
LoLA @schmidt_lola_2000,
Marcie,
GreatSPN,
LTSmin,
pnmc,
and mcc4mcc, which is a meta-tool that dispatches queries to other tools.

They scale through reductions @thierry-mieg_structural_2020, decision diagrams @couvreur_hierarchical_2005 and partial-order methods @valmari_stubborn_1991.
Most are written in C or C++ and take their query as a temporal-logic formula.

*Constraint-based analyzers* (six tools) formulate Petri net properties in a constraint language and solve them with a SAT or SMT solver:
SMPT,
Petrinizer @esparza_smt-based_2014,
QCover,
Inequalizer,
mist,
and bounded-model-checker-for-petri-nets.
Their results tend to include independently verifiable certificates as a by-product of the constraint solving process.

*Workflow and process-mining tools* (eight tools) are designed to analyze soundness of workflow nets:
WoPeD @freytag_woped_2017,
ProM with Woflan
PM4Py,
Wolfgang,
jBPT,
YAPNE,
wasm4pm,
and bpmn2pnml-ltl-analyzer.
They answer a single composite query: "Is this workflow net sound?" and can point out particular
transitions or places which violate soundness.

*Libraries and small tools in language ecosystems* (seven tools) offer analysis as a function call from Python, Rust or Go:
SNAKES,
petritub,
go-pflow,
isene/petri,
visualpetri-mcp,
libpetri,
and petricheck.
They are generally limited in scope (capped state space, only one property, etc.) and their documentation is overall sparse.

*Teaching, desktop and web analysers* (seven tools) place a graphical user interface in front of classical algorithms (mostly Karp-Miller coverability tree @karp_parallel_1969):
APT,
Charlie @heiner_charlie_2015,
the MATLAB Petri Net Toolbox,
GPenSIM,
Hippo-CPS,
PnAT,
and petri-nets-tool-reactjs.

== Findings

I assigned each tool a score on a four-point scale for each requirement based on my reading of the documentation
and source code: "✓" = fully meets the requirement, "(✓)" = partly meets the requirement, "✗" = does not meet the
requirement, and "?" = not determined.
These scores are a subjective, qualitative assessment of the tools' capabilities and can be disputed.

#figure(
  kind: table,
  caption: "Table of the 37 tools surveyed, their properties, and the scores for each requirement.",
  text(size: 5.5pt, table(
    columns: (auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto),
    align: (left, left, left, left, center, center, center, center, center, center, center, left, left),
    inset: 2.5pt,
    stroke: 0.4pt + luma(170),
    table.header([*\#*], [*Tool*], [*Properties decided*], [*Dispatch by net class*], [*R1*], [*R2*], [*R3*], [*R4*], [*R5*], [*R6*], [*R7*], [*R8*], [*Last release or commit*]),
    table.cell(colspan: 13, fill: luma(235))[*Performance-oriented model checkers*],
    [1], [TAPAAL / verifypn], [R, CTL, LTL (query language)], [by query type], [(✓)], [✗], [(✓)], [(✓)], [✓], [method, witness], [(✓)], [✓], [2026-06],
    [2], [TINA], [B, D, L, R, C (graphs, model checker)], [by net features (time, priorities)], [✓], [✓], [✓], [(✓)], [✓], [method, witness, culprits], [✗], [✗], [2026-04],
    [3], [ITS-Tools], [R, D, B (MCC examinations)], [not documented], [(✓)], [✗], [✓], [✓], [✓], [method, witness], [(✓)], [(✓)], [2026-09],
    [5], [LoLA], [B, D, L, R, C (graph)], [per property only], [✓], [✗], [✓], [✗], [✓], [method, witness, theorem], [(✓)], [✗], [2020-07],
    [6], [Marcie], [D, L, R, CTL; B only as degree], [not documented], [(✓)], [✗], [(✓)], [(✓)], [✓], [method, witness, culprits], [✗], [✗], [2017-06],
    [7], [GreatSPN], [B, D, L, CTL (P/T part of a stochastic suite)], [not documented], [(✓)], [✗], [✗], [(✓)], [✓], [method, culprits], [(✓)], [✓], [2026-02],
    [23], [mcc4mcc], [delegates to MCC tools (all examinations)], [learned from net characteristics], [(✓)], [(✓)], [✗], [✗], [✓], [not documented], [(✓)], [(✓)], [2018-10],
    [24], [LTSmin], [R, D (L only as formula)], [not documented], [(✓)], [✗], [(✓)], [(✓)], [✓], [witness, method], [✓], [✓], [2024-10],
    [25], [pnmc], [D, dead transitions], [not documented], [✓], [✗], [✓], [(✓)], [✓], [witness, method], [(✓)], [(✓)], [2026-04 (bot)],
    table.cell(colspan: 13, fill: luma(235))[*Constraint-based checkers (SMT, SAT)*],
    [4], [SMPT], [R, D, quasi-L, C], [not documented], [(✓)], [(✓)], [✓], [(✓)], [✓], [method, witness, certificate], [(✓)], [(✓)], [2025-06],
    [10], [Petrinizer], [B, D, R (unreachability proof)], [no], [(✓)], [(✓)], [(✓)], [(✓)], [(✓)], [method, certificate, culprits], [(✓)], [(✓)], [2020-11],
    [11], [QCover], [C], [not documented], [(✓)], [(✓)], [✗], [✗], [(✓)], [method], [(✓)], [(✓)], [2019-08],
    [12], [Inequalizer], [R, C (disproof only)], [not documented], [(✓)], [(✓)], [(✓)], [✗], [(✓)], [method, certificate, theorem], [(✓)], [(✓)], [2020-05],
    [31], [bounded-model-checker-for-petri-nets], [D on 1-safe nets only], [not documented], [(✓)], [✗], [(✓)], [✗], [✗], [method], [(✓)], [✗], [2018-12],
    [36], [mist], [C], [user picks the algorithm], [(✓)], [✗], [✓], [✗], [(✓)], [method, witness], [(✓)], [(✓)], [2026-09 (bot)],
    table.cell(colspan: 13, fill: luma(235))[*Workflow and process-mining tools (soundness)*],
    [15], [PM4Py], [workflow soundness (B, D, L)], [in code, not documented], [✓], [(✓)], [(✓)], [✓], [✗], [method, witness, culprits], [✓], [✓], [2026-10],
    [16], [ProM (Woflan)], [B, L, C and R graphs, soundness], [no], [(✓)], [✓], [✗], [(✓)], [✗], [method, theorem, witness, culprits], [✗], [✗], [2025-09],
    [17], [WoPeD], [soundness (B, D, L), C and R graphs], [no], [✗], [✓], [(✓)], [✓], [✗], [method, culprits], [(✓)], [✓], [2026-05],
    [18], [Wolfgang], [B (in source)], [not documented], [✗], [✓], [✗], [✓], [✗], [not documented], [(✓)], [✗], [2017-12],
    [21], [YAPNE], [data-aware soundness (D, dead transitions)], [not documented], [(✓)], [✓], [✗], [✓], [✗], [method, witness, culprits], [(✓)], [✓], [2026-10],
    [22], [bpmn2pnml-ltl-analyzer], [D, livelock via LTL], [not documented], [✓], [✓], [✗], [(✓)], [✗], [not documented], [✓], [(✓)], [2026-07],
    [26], [jBPT], [B (own); R, L, soundness via bundled LoLA], [structure detected only], [✓], [(✓)], [✓], [✓], [✗], [culprits, method], [✓], [✓], [2026-08],
    [35], [wasm4pm], [workflow soundness, safeness], [detection only], [✓], [✓], [(✓)], [✓], [✗], [theorem, culprits], [(✓)], [(✓)], [2026-10],
    table.cell(colspan: 13, fill: luma(235))[*Libraries and small tools in language ecosystems*],
    [13], [SNAKES], [R (finite state graph)], [not documented], [✓], [✓], [✗], [(✓)], [✗], [method], [✓], [(✓)], [2024-06],
    [14], [petritub], [B, C, dead transitions], [not documented], [✓], [✓], [✗], [✗], [✗], [method], [✗], [✗], [2026-04],
    [27], [go-pflow], [D, B, quasi-L, R], [structural first, then exhaustive], [✓], [✓], [(✓)], [(✓)], [✗], [method, witness, theorem, certificate, culprits], [✓], [(✓)], [2026-10],
    [30], [isene/petri], [B, D (capped search)], [not documented], [(✓)], [✓], [(✓)], [✗], [✗], [culprits], [(✓)], [(✓)], [2026-10],
    [32], [visualpetri-mcp], [R, D (bounded graph)], [not documented], [✓], [✓], [✓], [✗], [✗], [witness, culprits], [(✓)], [(✓)], [2026-10],
    [33], [libpetri], [D, place bound, unreachability, mutual exclusion], [by net features], [✓], [(✓)], [✓], [✗], [✗], [method, witness, certificate, theorem], [✓], [(✓)], [2026-10],
    [34], [petricheck], [k-safeness; R, D, L via user CTL], [no], [✓], [✓], [(✓)], [(✓)], [✗], [culprits], [✓], [(✓)], [2026-02],
    table.cell(colspan: 13, fill: luma(235))[*Teaching, desktop and web analysers*],
    [8], [APT], [B, C, weak L], [not documented], [✓], [✓], [✓], [(✓)], [✗], [method, witness, culprits], [✓], [(✓)], [2026-02],
    [9], [Charlie], [B, L, C, dead states, R via CTL/LTL], [class detected, not a switch], [(✓)], [✓], [(✓)], [✗], [✗], [method, theorem, witness, culprits], [✗], [✗], [2017-06],
    [19], [Petri Net Toolbox], [B, L, D, C (omega tree)], [not documented], [(✓)], [(✓)], [✗], [✗], [✗], [method, culprits], [✗], [✗], [2025-09],
    [20], [Hippo-CPS], [B, L], [user chooses method], [✗], [?], [(✓)], [?], [(✓)], [method, culprits], [✗], [✗], [no date],
    [28], [petri-nets-tool-reactjs], [R, B, L, invariants, siphons, traps], [not documented], [✗], [✓], [(✓)], [✗], [✗], [none], [(✓)], [(✓)], [2023-02],
    [29], [PnAT], [B, weak termination, k-bound], [not documented], [✗], [✓], [(✓)], [(✓)], [✗], [witness], [(✓)], [✗], [2023-12],
    [37], [GPenSIM], [B, D, C, R (capped)], [user picks cotree vs cotreei], [✓], [(✓)], [(✓)], [✓], [✗], [method, culprits], [✗], [✗], [2025-08],
  )),
) <tab:landscape>

#figure(
  caption: [Number of the 37 tools that meet each requirement fully (dark), partly (medium) or not (light). R6 is not rated on a scale; see the traceability counts in the text.],
  {
    let total = 37
    let w = 9cm
    let seg(count, fill, txtcolor) = if count > 0 { box(width: w * count / total, height: 0.45cm, fill: fill, align(center + horizon, text(size: 7pt, fill: txtcolor)[#count])) }
    let bar(label, met, part, no) = grid(columns: (4.2cm, auto), column-gutter: 4pt, align: horizon,
      text(size: 8pt)[#label],
      stack(dir: ltr, seg(met, rgb("#2a6f4f"), white), seg(part, rgb("#9bc4ae"), black), seg(no, luma(225), black)))
    stack(dir: ttb, spacing: 5pt,
      bar([R1 programmatic access], 15, 17, 5),
      bar([R2 safe portability], 16, 10, 11),
      bar([R3 cost control], 10, 17, 10),
      bar([R4 model interchange], 8, 15, 14),
      bar([R5 performance evidence], 10, 5, 22),
      bar([R7 extensibility], 9, 20, 8),
      bar([R8 contribution-friendliness], 7, 18, 12),
    )
  },
) <fig:landscape-bars>

*R1, programmatic access.* Fifteen tools return structured data from a function call or command-line interface,
and seventeen tools can only be scripted by parsing textual output or reading exit codes.
Five tools provide no interface other than a GUI.
Among the performance-oriented model checkers, only TINA, LoLA and pnmc document a JSON output format;
the others print text lines (ITS-Tools, SMPT) or XML traces and exit codes (VerifyPN).
Libraries exist, but they sit mostly at the weak end of the spectrum.

*R2, safe portability.* Sixteen tools are written in a memory-safe language and do not depend on native code.
Seven of the nine performance-oriented checkers have a native C or C++ core and so sacrifice memory safety for performance
(this is not a judgement of the quality of their implementation).
TINA is implemented in Standard ML, which is memory-safe, and mcc4mcc containerizes other (not memory-safe) tools within Docker.
Four of the six constraint-based checkers depend on the Z3 SMT solver, which is written in C++ and has a history of
memory-safety issues @winterer_validating_2020 @park_generative_2021.
Of the libraries scored for R2, the Rust, Go, and Python libraries are at least memory-safe in principle, but I did not
audit their use of `unsafe` or of native dependencies.

*R3, cost control.* Ten tools allow the user to bound a query with a timeout, memory limit, state-space cap, or choice
of procedure. Seventeen offer partial control (a fixed cap, timeout, or depth bound).
Ten tools offer no cost control mechanism at all.

*R4, model interchange.* Eight tools read and write PNML.
LoLA documents that it deliberately uses a proprietary format instead of PNML.
Thirteen tools have a proprietary format only, or none documented.

*R5, performance and memory efficiency.* Ten tools have independent evidence from the Model Checking Contest;
five more have a published evaluation, mostly by their authors.
The remaining twenty-two tools have no performance evidence.

*R6, traceability to theory.* Twenty-eight tools can name the technique that decided a query,
eighteen can return a witness or counterexample,
nineteen name the net elements responsible for a negative result,
seven cite the theorem on which the answer rests,
and five can emit a certificate that can be independently verified.

*R7, extensibility.* Nine tools (APT, SNAKES, PM4Py, bpmn2pnml-ltl-analyzer, LTSmin, jBPT, go-pflow, libpetri and petricheck)
publish their source under an OSI-approved license and are either usable as a library or document an extension mechanism.
Twenty tools publish source but document neither a library interface nor an extension point, or lack a clear open license.
Among them, Petrinizer, Inequalizer, the bounded model checker and visualpetri-mcp state no license at all,
which gives no right to reuse or modify the code, and wasm4pm combines an Apache, an MIT and a commercial license.
Eight tools, including TINA, Charlie, Marcie and the Petri Net Toolbox, offer no source that I could find.

*R8, contribution-friendliness.* I counted five indicators for each tool:
a public repository with an issue tracker,
a documented build from a clean checkout with a standard toolchain,
automated tests or continuous integration,
contribution guidelines or developer documentation,
and commits by at least two distinct human authors between 2024-10-14 and 2026-10-14 (bots and AI accounts excluded).
Seven tools (TAPAAL/verifypn, GreatSPN, PM4Py, WoPeD, YAPNE, LTSmin and jBPT) meet at least four, including the first and the last.
Eighteen meet two or three, or lack one of the first and the last.
Twelve meet at most one indicator or have no public repository.
Only PM4Py, LTSmin and jBPT are rated as met on both R7 and R8, and each of them fails another requirement
(R5 for PM4Py and jBPT, R2 for LTSmin).

*Across the requirements,* no tool fully meets all of R1 to R5, R7 and R8.
The most requirements met in full is five of those seven (jBPT), followed by four (TINA, PM4Py and APT).
Only SMPT and Petrinizer fail none of the seven requirements outright, and both meet only some of them partly.

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
- The scoring is subjective and unverified; I did not ask the authors of the tools to confirm my assessment.
- "Not documented" means that I could not find documentation, not that the tool does not have it.
- Maintenance is judged by the date of the last release or commit, which may not reflect the actual state of the project.
