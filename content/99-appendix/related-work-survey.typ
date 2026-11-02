#let survey = "/related-work-survey/screening/"
#let counts = json(survey + "counts.json")
#let stage2 = csv(survey + "stage2.tsv", delimiter: "\t", row-type: dictionary)

// an integer with thousands separators ("10000+" keeps its plus)
#let num(x) = {
  let s = str(x)
  let plus = s.ends-with("+")
  let d = s.trim("+")
  let out = ""
  for (i, c) in d.clusters().enumerate() {
    if i > 0 and calc.rem(d.len() - i, 3) == 0 { out += "," }
    out += c
  }
  out + if plus { "+" } else { "" }
}
#let source-name = (
  S1: "Google Scholar", S2: "DBLP", S3a: "ACM Digital Library", S3b: "SpringerLink", S8: "PyPI",
  S4: "crates.io", S5: "GitHub", S6: "Hamburg tool database", S7: "MCC 2025 participants", S9: "Maven Central",
  O1: "other means", "stage 2 follow-up": "stage 2 follow-up",
)
// reason codes of the data, in the names used in the thesis
#let reason-name = (
  I1: "IC1", I2: "IC2", I3: "IC3", E0: "off topic", E1: "EC1", E2: "EC2", E3: "no documentation", E4: "duplicate",
  E5: "no tool presented",
)
#let reasons(s) = s.split(", ").map(c => reason-name.at(c, default: c)).join(", ")
#let query-string = (
  Q1: "Petri net analysis library",
  Q2: "Petri net structural analysis tool free-choice liveness boundedness",
  Q3: "workflow net soundness tool diagnostics",
  Q4: "Petri net coverability SMT CEGAR",
  Q5: "Petri net reachability certificate explanation",
  D1: "petri net analysis tool", D2: "petri net structural analysis", D3: "workflow net soundness",
  D4: "petri net coverability", D5: "petri net reachability certificate",
  P1: "petri net", P2: "petri net analysis",
)
#let small-table(columns, align, header, ..cells) = text(size: 8pt, table(
  columns: columns, align: align, inset: 3pt, stroke: 0.4pt + luma(170),
  table.header(..header.map(h => [*#h*])), ..cells,
))
#let words = ("no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve")
#let tool-name = stage2.filter(r => r.decision == "include").fold((:), (m, r) => { m.insert(r.tool_id, r.name); m })

== Related Work Survey Protocol <appendix-related-work-survey>

This appendix protocols the search and screening process of the related work survey in @related-work.
All searches were performed on 2026-10-14.
Every search record and decision with justification, the facts behind every rating,
and the scripts that derive the statistics and tables below are available in the thesis repository
#footnote(<thesis-repo>).

=== Search Log

The scholarly sources and PyPI were searched by hand with query strings fixed before the search.
Google Scholar, the ACM Digital Library and SpringerLink all index abstracts or full text, and received the query strings Q1 to Q5 verbatim.
DBLP indexes only titles, and received the shorter query strings D1 to D5.
The first 50 results per query were screened on relevance-ranked sources (Google Scholar, ACM Digital Library, SpringerLink, PyPI),
and all other sources were screened in their entirety.

#figure(
  kind: table,
  small-table(
    (auto, auto, 1fr, auto, auto, auto), (left, left, left, right, right, right),
    ("ID", "Source", "Search string", "Hits", "Screened", "Passed"),
    ..counts.scholarly_queries.map(q => {
      let (src, id) = q.id.split("-")
      (q.id, source-name.at(src), raw(query-string.at(id)), num(q.hits), str(q.screened), str(q.passed))
    }).flatten(),
  ),
  caption: [
    Query results of the scholarly sources and PyPI: hits, records screened at stage 1, and records passing stage 1.
  ],
) <tab-search-log-scholarly>

These queries yielded #num(counts.scholarly_rows_screened) rows and #num(counts.scholarly_unique) unique records after deduplication.
A record found by several queries is counted once, at its first occurrence.

Software registries and tool lists were queried through their public interfaces.
A query returning more than 200 hits was narrowed by property keywords, and the first 200 hits were screened.

#let reg = counts.registry_hits
#let s1 = counts.stage1
#figure(
  kind: table,
  caption: [Queries of the software registries and tool lists: hits, records screened at stage 1 (after merging duplicates), and records passing stage 1.],
  small-table(
    (auto, 1fr, auto, auto, auto), (left, left, right, right, right),
    ("Source", "Query", "Hits", "Screened", "Passed"),
    source-name.S4, reg.S4.map(q => raw(q.query)).join[, ], reg.S4.map(q => num(q.hits)).join(" / "), str(s1.S4.screened), str(s1.S4.passed),
    ..reg.S5.map(q => (
      source-name.S5 + if q.id != "" [ (#q.id)], raw(q.query), num(q.hits),
      if q.id == "" [stopped (200-hit rule)] else if q.id == "GH-A" { table.cell(rowspan: 2)[#s1.S5.screened] },
      if q.id == "" [--] else if q.id == "GH-A" { table.cell(rowspan: 2)[#s1.S5.passed] },
    ).filter(c => c != none)).flatten(),
    source-name.S6, [complete list], num(s1.S6.screened), str(s1.S6.screened), str(s1.S6.passed),
    source-name.S7, [participant table], num(s1.S7.screened), str(s1.S7.screened), str(s1.S7.passed),
    source-name.S9, reg.S9.map(q => raw(q.query)).join[, ], reg.S9.map(q => num(q.hits)).join(" / "), [#s1.S9.screened projects], str(s1.S9.passed),
  ),
) <tab-search-log-registries>

=== Selection

#let order = ("S1", "S2", "S3a", "S3b", "S8", "S4", "S5", "S6", "S7", "S9")
#figure(
  kind: table,
  caption: [
    Records screened per source, records passing stage 1, stage-1 exclusions by reason,
    and the number of included tools for which a record of the source reached stage 2.
    A tool found by several sources is counted for each of them.
  ],
  small-table(
    (auto, auto, auto, 1fr, auto), (left, right, right, left, right),
    ("Source", "Screened", "Passed stage 1", "Excluded at stage 1", "Included tools found"),
    ..order.map(s => (
      source-name.at(s), num(s1.at(s).screened), str(s1.at(s).passed),
      s1.at(s).excluded.pairs().map(((k, v)) => [#v: #reason-name.at(k, default: k)]).join[, \ ],
      str(s1.at(s).found_included),
    )).flatten(),
  ),
) <tab-selection>

Every record passing stage 1 was screened at stage 2 by reading its documentation (README, manual, or paper).
Records describing the same tool were merged.
Of the #counts.stage2.records records screened at stage 2, #counts.stage2.included tools were included.

#let only-o1 = counts.stage2.found_only_by_source.at("O1", default: ()).map(i => tool-name.at(str(i)))
#let follow-up = counts.stage2.found_only_by_source.at("stage 2 follow-up", default: ()).map(i => tool-name.at(str(i)))
#upper(words.at(only-o1.len()).first())#words.at(only-o1.len()).slice(1) tools were found only by other means (#only-o1.join(", ", last: " and ")),
and #follow-up.join(", ") was found while following up a record of its predecessor at stage 2.

=== Amendments

@tab-survey-protocol-amendments summarizes the amendments to the survey protocol made during the screening process.

#figure(
  kind: table,
  caption: [Amendments to the survey protocol.],
  small-table(
    (auto, auto, auto), (left, left, left),
    ("#", "Change", "Reason and effect"),
    [A1],
    [Scholarly sources run by hand; DBLP receives title-style strings D1 to D5; ACM Digital Library and SpringerLink treated as separate sources],
    [DBLP indexes titles only. Fixed before the scholarly searches.],

    [A2],
    [Exclusion code "no tool presented" added: a publication that presents no obtainable tool],
    [Most scholarly records are theory or application papers, which are not "off topic" but do not present an artifact for the survey. No records needed to be re-screened.],

    [A3],
    [Relevance-ranked sources (Google Scholar, ACM Digital Library, SpringerLink, PyPI): first 50 results per query],
    [Both PyPI queries report more than 10,000 projects, so the 200-hit narrowing rule could not be met.],

    [A4],
    [*IC3 widened from 5 to 10 years* (last release or commit on or after 2016-10-14)],
    [Made after the registries had been screened, prompted by LoLA (last release 2020-07-08): Petri net analyzers are finished research artifacts that are rarely updated. 29 GitHub repositories and one PyPI package excluded under the old rule were re-screened against IC1; only mcc4mcc entered. Five papers from 1986 to 1990 remained excluded.],

    [A5],
    [R6 recorded as a list of evidence tags (method, witness, certificate, theorem, culprits) instead of levels],
    [A scale could not be applied consistently to all tools. The rating follows from the tags (@tab-rating-rules).],

    [A6],
    [IC1 made explicit: a tool must document a decision procedure for one of the five properties on nets that can be plain place/transition nets],
    [Stage 2 produced eight borderline cases; one rule replaced case-by-case judgments. Six were excluded (Kong, DPN Verifier, SkiNet, ePNK, travis, PN-Suite), two included (YAPNE, Hippo-CPS).],
  ),
) <tab-survey-protocol-amendments>

=== Rating Rules

Ratings are given only for what a documentation page, manual or source file shows; "not documented" counts as not met.

#figure(
  kind: table,
  caption: [Rating rules for the requirements.],
  small-table(
    (auto, auto, auto, auto), (left, left, left, left),
    ("Req.", "✓ met", "(✓) partly met", "✗ not met"),
    [R1], [library API with typed results, or documented structured output (JSON, XML)], [command line whose only machine-readable channel is plain text or exit codes], [graphical interface only, or no interface documented],
    [R2], [analysis code in a memory-safe language, no native engine or external binary], [memory-safe language, but needs an external native solver, runtime or platform], [analysis engine in C or C++],
    [R3], [documented per-query timeout, state or memory budget, or a choice of cheaper procedures], [only a fixed cap, a per-phase timeout, a depth bound, or options without a budget], [nothing documented],
    [R4], [PNML import and export documented], [PNML in one direction only, or other exchange formats only], [own format only, or none documented],
    [R5], [participation in the Model Checking Contest (any year), or an independent benchmark], [published benchmark by the authors, or a third-party comparison], [none found],
    [R6], [can return a witness or a certificate], [can only name the net elements responsible (culprits)], [neither; naming the method does not count],
    [R7], [source under an OSI-approved license, and usable as a library or with a documented extension mechanism], [source available without a library interface or extension point, or an extension point under a license that does not permit modification], [binary only, web service only, or no source],
    [R8], [at least four of the indicators (a) to (e) below, including (a) and (e)], [two or three indicators, or only one of (a) and (e)], [at most one indicator, or no public repository],
  ),
) <tab-rating-rules>

The indicators for R8 are:
- (a) a public repository with an issue tracker;
- (b) a build from a clean checkout with a standard, documented toolchain;
- (c) automated tests or continuous integration in the repository;
- (d) contribution guidelines or developer documentation; and
- (e) commits by at least two distinct human authors between 2024-10-14 and 2026-10-14.
The rules for R7 and R8 were fixed on 2026-10-20, before their data was collected.

=== By-Hand Re-Coding

#let ag = counts.agreement
#let pct(a, b) = str(calc.round(100 * a / b, digits: 1)) + "%"
A random 20% sample of the stage-1 records were re-coded by hand, without seeing the first coding
(@tab-survey-agreement).

#figure(
  kind: table,
  caption: [Agreement on pass or not pass at stage 1 between the first coding and a blind second coding.],
  small-table(
    (auto, auto, auto, auto), (left, right, right, left),
    ("Sample", "Records", "Agreement", "Notes"),
    [Scholarly sources (seed 20261014)], str(ag.scholarly.sample),
    [#pct(ag.scholarly.agree, ag.scholarly.sample) (κ = #ag.scholarly.kappa)],
    [Reasons agree for #pct(ag.scholarly.agree_e0_e5_merged, ag.scholarly.sample) when "off topic" and "no tool presented" are counted together (A7).],
    [Registries and tool lists (seed 20261014; Hamburg database: seed 20261015)], [#ag.others.coded of #ag.others.sample coded],
    pct(ag.others.agree, ag.others.coded),
    [#(ag.others.sample - ag.others.coded) records could not be coded because the site did not load.],
  ),
) <tab-survey-agreement>