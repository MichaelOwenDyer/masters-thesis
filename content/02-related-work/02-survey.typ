#import "../lib.typ": *

== Survey Results

The 37 tools fall into five categories based on their primary purpose:

*Performance-oriented model checkers* (9) are built with LTL and CTL properties in mind:
- TAPAAL/VerifyPN #footnote[https://www.tapaal.net/, last accessed: 2026-10-27] @jensen_tapaal_2016,
- TINA #footnote[https://projects.laas.fr/tina/, last accessed: 2026-10-25] @berthomieu__tool_2004 @vernadat_time_2006,
- ITS-Tools #footnote[https://lip6.github.io/ITSTools-web/, last accessed: 2026-10-27] @thierry-mieg_symbolic_2015,
- LoLA #footnote[https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/, last accessed: 2026-10-27] @schmidt_lola_2000,
- Marcie #footnote[https://www-dssz.informatik.tu-cottbus.de/DSSZ/Software/Marcie, last accessed: 2026-10-14],
- GreatSPN #footnote[https://github.com/greatspn/SOURCES, last accessed: 2026-10-14],
- LTSmin #footnote[https://github.com/utwente-fmt/ltsmin, last accessed: 2026-10-14],
- pnmc #footnote[https://github.com/ahamez/pnmc, last accessed: 2026-10-14], and
- mcc4mcc #footnote[https://github.com/cui-unige/mcc4mcc, last accessed: 2026-10-14], which is a meta-tool that dispatches queries to other tools.
They scale through reductions @thierry-mieg_structural_2020, decision diagrams @couvreur_hierarchical_2005 and partial-order methods @valmari_stubborn_1991.
Most are written in C or C++ and accept queries in the form of temporal-logic formulae.

*Constraint-based analyzers* (6) formulate Petri net properties in a constraint language and dispatch to a SAT or SMT solver:
- SMPT #footnote[https://github.com/nicolasAmat/SMPT, last accessed: 2026-10-14],
- Petrinizer #footnote[https://github.com/cryptica/pnerf, last accessed: 2026-10-14] @esparza_smt-based_2014,
- QCover #footnote[https://github.com/michaelblondin/qcover, last accessed: 2026-10-14],
- Inequalizer #footnote[https://github.com/florianfurbach/Inequalizer, last accessed: 2026-10-14],
- mist #footnote[https://github.com/pierreganty/mist, last accessed: 2026-10-14], and
- bounded-model-checker-for-petri-nets #footnote[https://github.com/tatsuhirotsuchiya/bounded-model-checker-for-petri-nets, last accessed: 2026-10-14].
Their results tend to include independently verifiable certificates as a by-product of the constraint solving process.

*Workflow and process-mining tools* (8) are designed to analyze soundness of workflow nets
#footnote[
  A workflow net is a Petri net exhibiting
  a single source place (with an empty preset) which can reach every other node, and
  a single sink place (with an empty postset) which every other node can reach @van_der_aalst_verification_1997.
  They are commonly used to model and analyze business processes.
]:
- WoPeD #footnote[https://github.com/woped/WoPeD, last accessed: 2026-10-14] @freytag_woped_2017,
- ProM with Woflan #footnote[https://github.com/promworkbench, last accessed: 2026-10-14]
- PM4Py #footnote[https://github.com/process-intelligence-solutions/pm4py, last accessed: 2026-10-14],
- Wolfgang #footnote[https://github.com/iig-uni-freiburg/WOLFGANG, last accessed: 2026-10-27],
- jBPT #footnote[https://github.com/jbpt/codebase, last accessed: 2026-10-14],
- YAPNE #footnote[https://github.com/chimenkamp/YAPNE-Yet-Another-Petri-Net-Editor, last accessed: 2026-10-14],
- wasm4pm #footnote[https://github.com/seanchatmangpt/wasm4pm, last accessed: 2026-10-14], and
- bpmn2pnml-ltl-analyzer #footnote[https://github.com/Research-MCI/BPMN2PNML-LTL-Analyzer, last accessed: 2026-10-14].
They answer a single composite query: "Is this workflow net sound
#footnote[
  Soundness is a composite property of workflow nets requiring
  (1) there is exactly one reachable deadlock (final marking) which puts one token in the end place and no tokens elsewhere,
  (2) the final marking is reachable from every reachable marking, and
  (3) every transition is at least #lv(1)-live in the initial marking.
]?"
and can point out particular transitions or places which violate soundness.

*Libraries and small tools in language ecosystems* (7) offer analysis as a function call from Python, Rust or Go:
- SNAKES #footnote[https://github.com/fpom/snakes, last accessed: 2026-10-14],
- petritub #footnote[https://control.gitlab-pages.tu-berlin.de/discrete-event-systems/bsc-thesis-code/petri-tub-max-plus-automata, last accessed: 2026-10-14],
- go-pflow #footnote[https://github.com/pflow-xyz/go-pflow, last accessed: 2026-10-14],
- isene/petri #footnote[https://github.com/isene/petri, last accessed: 2026-10-14],
- visualpetri-mcp #footnote[https://github.com/everlnggg/visualpetri-mcp, last accessed: 2026-10-14],
- libpetri #footnote[https://github.com/debe/libpetri, last accessed: 2026-10-14], and
- petricheck #footnote[https://github.com/erwanM974/petricheck, last accessed: 2026-10-14].
They are generally limited in scope (capped state space, only one property, etc.) and their documentation is overall sparse.

*Teaching, desktop and web analysers* (7) place a graphical user interface in front of classical algorithms (mostly Karp-Miller coverability tree @karp_parallel_1969):
- APT #footnote[https://github.com/CvO-Theory/apt, last accessed: 2026-10-14],
- Charlie #footnote[https://www-dssz.informatik.tu-cottbus.de/DSSZ/Software/Charlie, last accessed: 2026-10-14] @heiner_charlie_2015,
- the MATLAB Petri Net Toolbox #footnote[https://www.pntool.ac.tuiasi.ro/, last accessed: 2026-10-14],
- GPenSIM #footnote[https://www.davidrajuh.net/gpensim, last accessed: 2026-10-14],
- Hippo-CPS #footnote[https://hippo.uz.zgora.pl, last accessed: 2026-10-14],
- PnAT #footnote[https://github.com/dbera/Petri-net-Analysis-Tools, last accessed: 2026-10-14], and
- petri-nets-tool-reactjs #footnote[https://github.com/albertovalerio/petri-nets-tool-reactjs, last accessed: 2026-10-14].

== Findings

I assigned each tool a score on a four-point scale for each requirement based on my reading of the documentation
and source code: "✓" = fully meets the requirement, "(✓)" = partly meets the requirement, "✗" = does not meet the
requirement, and "?" = not determined.
These scores are a subjective, qualitative assessment of the tools' capabilities and can be disputed.
R6 is rated by what a tool returns as evidence for an answer:
"✓" if it can return a witness (for example a firing sequence or a counterexample) or an independently verifiable certificate,
"(✓)" if it can only name the net elements responsible for a negative result,
and "✗" if it offers neither (naming the technique is recorded in the table but does not count).
The column "Dispatch by net class" records whether the documentation says that the tool selects its procedure by the structure of the net;
it is not a rated requirement and is discussed separately below.

#show figure: set block(breakable: true)
#figure(
  kind: table,
  caption: "Table of the 37 tools surveyed, their properties, and the scores for each requirement.",
  text(size: 8pt, table(
    columns: (auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto),
    align: (left, left, left, left, center, center, center, center, center, center, left, center, center, left),
    inset: 2.5pt,
    stroke: 0.4pt + luma(170),
    table.header([*\#*], [*Tool*], [*Properties decided*], [*Dispatch by net class*], [*R1*], [*R2*], [*R3*], [*R4*], [*R5*], [*R6*], [*Evidence offered*], [*R7*], [*R8*], [*Last release or commit*]),
    table.cell(colspan: 14, fill: luma(235))[*Performance-oriented model checkers*],
    [1], [TAPAAL / verifypn], [R, CTL, LTL (query language)], [by query type], [(✓)], [✗], [(✓)], [(✓)], [✓], [✓], [method, witness], [(✓)], [✓], [2026-06],
    [2], [TINA], [B, D, L, R, C (graphs, model checker)], [by net features (time, priorities)], [✓], [✓], [✓], [(✓)], [✓], [✓], [method, witness, culprits], [✗], [✗], [2026-04],
    [3], [ITS-Tools], [R, D, B (MCC examinations)], [not documented], [(✓)], [✗], [✓], [✓], [✓], [✓], [method, witness], [(✓)], [(✓)], [2026-09],
    [5], [LoLA], [B, D, L, R, C (graph)], [per property only], [✓], [✗], [✓], [✗], [✓], [✓], [method, witness, theorem], [(✓)], [✗], [2020-07],
    [6], [Marcie], [D, L, R, CTL; B only as degree], [not documented], [(✓)], [✗], [(✓)], [(✓)], [✓], [✓], [method, witness, culprits], [✗], [✗], [2017-06],
    [7], [GreatSPN], [B, D, L, CTL (P/T part of a stochastic suite)], [not documented], [(✓)], [✗], [✗], [(✓)], [✓], [(✓)], [method, culprits], [(✓)], [✓], [2026-02],
    [23], [mcc4mcc], [delegates to MCC tools (all examinations)], [learned from net characteristics], [(✓)], [(✓)], [✗], [✗], [✓], [✗], [not documented], [(✓)], [(✓)], [2018-10],
    [24], [LTSmin], [R, D (L only as formula)], [not documented], [(✓)], [✗], [(✓)], [(✓)], [✓], [✓], [witness, method], [✓], [✓], [2024-10],
    [25], [pnmc], [D, dead transitions], [not documented], [✓], [✗], [✓], [(✓)], [✓], [✓], [witness, method], [(✓)], [(✓)], [2026-04 (bot)],
    table.cell(colspan: 14, fill: luma(235))[*Constraint-based checkers (SMT, SAT)*],
    [4], [SMPT], [R, D, quasi-L, C], [not documented], [(✓)], [(✓)], [✓], [(✓)], [✓], [✓], [method, witness, certificate], [(✓)], [(✓)], [2025-06],
    [10], [Petrinizer], [B, D, R (unreachability proof)], [no], [(✓)], [(✓)], [(✓)], [(✓)], [(✓)], [✓], [method, certificate, culprits], [(✓)], [(✓)], [2020-11],
    [11], [QCover], [C], [not documented], [(✓)], [(✓)], [✗], [✗], [(✓)], [✗], [method], [(✓)], [(✓)], [2019-08],
    [12], [Inequalizer], [R, C (disproof only)], [not documented], [(✓)], [(✓)], [(✓)], [✗], [(✓)], [✓], [method, certificate, theorem], [(✓)], [(✓)], [2020-05],
    [31], [bounded-model-checker-for-petri-nets], [D on 1-safe nets only], [not documented], [(✓)], [✗], [(✓)], [✗], [✗], [✗], [method], [(✓)], [✗], [2018-12],
    [36], [mist], [C], [user picks the algorithm], [(✓)], [✗], [✓], [✗], [(✓)], [✓], [method, witness], [(✓)], [(✓)], [2026-09 (bot)],
    table.cell(colspan: 14, fill: luma(235))[*Workflow and process-mining tools (soundness)*],
    [15], [PM4Py], [workflow soundness (B, D, L)], [in code, not documented], [✓], [(✓)], [(✓)], [✓], [✗], [✓], [method, witness, culprits], [✓], [✓], [2026-10],
    [16], [ProM (Woflan)], [B, L, C and R graphs, soundness], [no], [(✓)], [✓], [✗], [(✓)], [✗], [✓], [method, theorem, witness, culprits], [✗], [✗], [2025-09],
    [17], [WoPeD], [soundness (B, D, L), C and R graphs], [no], [✗], [✓], [(✓)], [✓], [✗], [(✓)], [method, culprits], [(✓)], [✓], [2026-05],
    [18], [Wolfgang], [B (in source)], [not documented], [✗], [✓], [✗], [✓], [✗], [✗], [not documented], [(✓)], [✗], [2017-12],
    [21], [YAPNE], [data-aware soundness (D, dead transitions)], [not documented], [(✓)], [✓], [✗], [✓], [✗], [✓], [method, witness, culprits], [(✓)], [✓], [2026-10],
    [22], [bpmn2pnml-ltl-analyzer], [D, livelock via LTL], [not documented], [✓], [✓], [✗], [(✓)], [✗], [✗], [not documented], [✓], [(✓)], [2026-07],
    [26], [jBPT], [B (own); R, L, soundness via bundled LoLA], [structure detected only], [✓], [(✓)], [✓], [✓], [✗], [(✓)], [culprits, method], [✓], [✓], [2026-08],
    [35], [wasm4pm], [workflow soundness, safeness], [detection only], [✓], [✓], [(✓)], [✓], [✗], [(✓)], [theorem, culprits], [(✓)], [(✓)], [2026-10],
    table.cell(colspan: 14, fill: luma(235))[*Libraries and small tools in language ecosystems*],
    [13], [SNAKES], [R (finite state graph)], [not documented], [✓], [✓], [✗], [(✓)], [✗], [✗], [method], [✓], [(✓)], [2024-06],
    [14], [petritub], [B, C, dead transitions], [not documented], [✓], [✓], [✗], [✗], [✗], [✗], [method], [✗], [✗], [2026-04],
    [27], [go-pflow], [D, B, quasi-L, R], [structural first, then exhaustive], [✓], [✓], [(✓)], [(✓)], [✗], [✓], [method, witness, theorem, certificate, culprits], [✓], [(✓)], [2026-10],
    [30], [isene/petri], [B, D (capped search)], [not documented], [(✓)], [✓], [(✓)], [✗], [✗], [(✓)], [culprits], [(✓)], [(✓)], [2026-10],
    [32], [visualpetri-mcp], [R, D (bounded graph)], [not documented], [✓], [✓], [✓], [✗], [✗], [✓], [witness, culprits], [(✓)], [(✓)], [2026-10],
    [33], [libpetri], [D, place bound, unreachability, mutual exclusion], [by net features], [✓], [(✓)], [✓], [✗], [✗], [✓], [method, witness, certificate, theorem], [✓], [(✓)], [2026-10],
    [34], [petricheck], [k-safeness; R, D, L via user CTL], [no], [✓], [✓], [(✓)], [(✓)], [✗], [(✓)], [culprits], [✓], [(✓)], [2026-02],
    table.cell(colspan: 14, fill: luma(235))[*Teaching, desktop and web analysers*],
    [8], [APT], [B, C, weak L], [not documented], [✓], [✓], [✓], [(✓)], [✗], [✓], [method, witness, culprits], [✓], [(✓)], [2026-02],
    [9], [Charlie], [B, L, C, dead states, R via CTL/LTL], [class detected, not a switch], [(✓)], [✓], [(✓)], [✗], [✗], [✓], [method, theorem, witness, culprits], [✗], [✗], [2017-06],
    [19], [Petri Net Toolbox], [B, L, D, C (omega tree)], [not documented], [(✓)], [(✓)], [✗], [✗], [✗], [(✓)], [method, culprits], [✗], [✗], [2025-09],
    [20], [Hippo-CPS], [B, L], [user chooses method], [✗], [?], [(✓)], [?], [(✓)], [(✓)], [method, culprits], [✗], [✗], [no date],
    [28], [petri-nets-tool-reactjs], [R, B, L, invariants, siphons, traps], [not documented], [✗], [✓], [(✓)], [✗], [✗], [✗], [none], [(✓)], [(✓)], [2023-02],
    [29], [PnAT], [B, weak termination, k-bound], [not documented], [✗], [✓], [(✓)], [(✓)], [✗], [✓], [witness], [(✓)], [✗], [2023-12],
    [37], [GPenSIM], [B, D, C, R (capped)], [user picks cotree vs cotreei], [✓], [(✓)], [(✓)], [✓], [✗], [(✓)], [method, culprits], [✗], [✗], [2025-08],
  )),
) <tab:landscape>

#show figure: set block(breakable: false)
#figure(
  {
    let total = 37
    let w = 9cm
    let seg(count, fill, txtcolor) = if count > 0 { box(width: w * count / total, height: 0.45cm, fill: fill, align(center + horizon, text(size: 7pt, fill: txtcolor)[#count])) }
    let bar(label, met, part, no) = grid(columns: (4.2cm, auto), column-gutter: 4pt, align: horizon,
      text(size: 8pt)[#label],
      stack(dir: ltr, seg(met, luma(50), white), seg(part, luma(170), black), seg(no, luma(225), black)))
    stack(dir: ttb, spacing: 5pt,
      bar([R1 programmatic access], 15, 17, 5),
      bar([R2 safe portability], 16, 10, 11),
      bar([R3 cost control], 10, 17, 10),
      bar([R4 model interchange], 8, 15, 14),
      bar([R5 performance evidence], 10, 5, 22),
      bar([R6 evidence with answers], 20, 9, 8),
      bar([R7 extensibility], 9, 20, 8),
      bar([R8 approachability], 7, 18, 12),
    )
  },
  caption: [
    Number of the 37 tools that meet each requirement fully (dark), partly (medium) or not (light).
    The ratings for R6 follow the evidence rule given above.
  ],
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

*R6, evidence with answers.* Twenty tools can return a witness or counterexample (eighteen tools) or an independently
verifiable certificate (five tools, three of which also return a witness).
Nine more can only name the net elements responsible for a negative result, and eight offer no evidence beyond the answer.

*R7, extensibility.* Nine tools (APT, SNAKES, PM4Py, bpmn2pnml-ltl-analyzer, LTSmin, jBPT, go-pflow, libpetri and petricheck)
publish their source under an OSI-approved license and are either usable as a library or document an extension mechanism.
Twenty tools publish source but document neither a library interface nor an extension point, or lack a clear open license.
Among them, Petrinizer, Inequalizer, the bounded model checker and visualpetri-mcp state no license at all,
which gives no right to reuse or modify the code, and wasm4pm combines an Apache, an MIT and a commercial license.
Eight tools, including TINA, Charlie, Marcie and the Petri Net Toolbox, offer no source that I could find.

*R8, approachability.* This rating covers only the contributor side of the requirement, through observable repository indicators;
it does not measure how easy a tool is to use, for people or for AI agents, and it is the softest rating of the survey.
I counted five indicators for each tool:
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

*Across the requirements,* no tool fully meets all of R1 to R8.
The most requirements met in full is five of the eight (TINA, PM4Py, jBPT and APT), followed by four (ITS-Tools, LoLA, LTSmin and pnmc).
Only SMPT and Petrinizer fail none of the eight requirements outright, and both meet only some of them partly.

== Dispatching by net structure

The answer to RQ2 depends on whether tools choose a decision procedure by the structure of the net, so I looked at what the documentation says about this.
Most tools do not say anything: for twenty-one tools I found no documentation of any selection, and for four more the documentation states
explicitly that there is none.
Some tools select by something other than the net structure: TAPAAL chooses by query type, LoLA applies a procedure per property,
and the contest meta-tool mcc4mcc chooses among other tools using a learned model of net characteristics.
In a few tools the user makes the choice (mist, Hippo-CPS, GPenSIM).
Only a handful of tools document anything like a selection by structure:
TINA by net features (time and priorities),
go-pflow by structural checks before exhaustive search,
and libpetri by net features;
jBPT, Charlie and wasm4pm detect the structure of the net but do not use it to switch procedures.
No tool documents a dispatch that spans the net classes (for example free-choice, marked graph and state machine) and cost-bounded fallbacks.
These counts depend on the verification noted with the table.
It is a gap in documentation as much as in implementation, but a user can only rely on what is documented.

== The gap

The requirements pull in different directions in the surveyed tools.
The tools with performance evidence (R5) mostly give up safe portability or extensibility:
of the ten tools with independent contest results, only TINA is rated fully on R2, and TINA publishes no source and offers no
contribution path (R7 and R8 not met), while LoLA fails R2 and R4, and LTSmin fails R2.
In the other direction, the libraries that provide programmatic access, extensibility and at least partly safe portability (SNAKES, go-pflow, libpetri and petricheck)
have no performance evidence (R5 is not met by any of the seven libraries), and their cost control is at best partial for three of them
and absent for SNAKES.
Workflow tools and teaching tools return the most helpful evidence, but they answer a narrow set of questions on small nets and show no performance evidence either.
Finally, almost no tool documents choosing its procedure by the structure of the net, so a user who wants the cheapest applicable procedure has to know the
theory and pick the tool, or the option, themselves.
This is the gap that `petrivet` aims to fill: a library that is programmatically accessible, memory-safe and extensible,
that decides each question with the cheapest procedure applicable to the net, that lets the caller bound the cost, and that is measured against the
established tools in @evaluation.
