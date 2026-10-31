== Methodology

I followed a structured literature review protocol @kitchenham_systematic_2009 to identify relevant Petri net analysis tools.
The protocol was fixed before the survey was conducted.

I searched four sources for academic papers:
- Google Scholar #footnote[https://scholar.google.com, last accessed: 2026-10-27],
- DBLP #footnote[https://dblp.org, last accessed: 2026-10-14],
- the ACM Digital Library #footnote[https://dl.acm.org, last accessed: 2026-10-14], and
- SpringerLink #footnote[https://link.springer.com, last accessed: 2026-10-14];

three sources for software packages:
- crates.io #footnote[https://crates.io, last accessed: 2026-10-14],
- PyPI #footnote[https://pypi.org, last accessed: 2026-10-14], and
- GitHub #footnote[https://github.com, last accessed: 2026-10-14];

and three sources for curated lists of Petri net tools:
- the Hamburg Petri Nets Tool Database #footnote[https://www2.informatik.uni-hamburg.de/tgi/PetriNets/tools/db.html, last accessed: 2026-10-14], and
- the Model Checking Contest (MCC) participant lists for
  2025 #footnote[https://mcc.lip6.fr/2025/results.php, last accessed: 2026-10-14] and
  2026 #footnote[https://mcc.lip6.fr/2026/results.php, last accessed: 2026-10-14] $ $
  #footnote[
    An early version of `petrivet` participated in the 2026 MCC
    (revision `c5af7627d5e016c4c7294a62db7591379e875e0a`, committed 25.05.2026).
    It placed last in every examination it entered,
    but the version predated the majority of `petrivet`'s current procedures
    and is not representative of the tool evaluated in this thesis.
  ]

In addition, the tools already known through prior experience were included in the survey.
A set of search queries was defined for each source.
For relevance-ranked sources (Google Scholar, ACM DL, SpringerLink, PyPI), the first 50
results per query were screened; all other sources were screened in their entirety.

A record entered the review in two stages.

At stage 1, search results were roughly filtered for relevance based on title, description/abstract, and metadata.
At stage 2, the remaining records were filtered for relevance by reading the documentation and/or source code of the tool.

A tool was included if:
- its documentation mentions a decision procedure for place/transition nets
  (or nets which can be interpreted as such) for at least one of
  boundedness, deadlock-freedom, liveness, reachability, or coverability (Inclusion Criterion 1, IC1);
- it is at least partially documented and available for download (IC2); and
- its last release or commit is dated within ten years of the survey, 2016-10-14 or later (IC3).

Pure editors and simulators without analysis capabilities were excluded (Exclusion Criterion 1, EC1),
as well as tools which only support colored, timed, stochastic, or other non-ordinary Petri nets (EC2).

IC3 was widened from 5 to 10 years after the survey had begun, triggered by LoLA @schmidt_lola_2000,
a prominent tool which was last released in 2020.
Previous exclusions based on IC3 were re-evaluated, and some tools were added back to the survey.
This and other protocol amendments are documented in @related-work-protocol-amendments.

The 785 records collected from the ten sources were filtered in stage 1 with the help of Claude Sonnet 5.5,
and a random sample of 20% (157) of the records was filtered by hand to verify the quality of the filtering.
Agreement between the two filtering methods was 94.3%, and Cohen's kappa was a moderate 0.50, which is explained
by the low pass rate (most results did not contain a relevant tool).
No disagreement was responsible for adding or removing a tool from stage 2.
Forty-six records passed stage 1 and were filtered by hand in stage 2,
resulting in 37 tools which finally entered the survey.
