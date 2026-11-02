#set raw(lang: "rust")

== Assessment against the Remaining Requirements <eval-requirements>

This section grades `petrivet` on all eight requirements R1 to R8 that were established in @motivation
with the rubric that was applied to the surveyed tools in @related-work,
so that its rating can be placed in @tab-tool-survey.

This grading cannot be independent: I wrote both the rubric and `petrivet`, and no second rater checked either (@related-work).
To limit this bias, three rules were followed.
First, every grade rests only on evidence that an independent reader can verify in `petrivet`'s repository.
The deeper knowledge of the implementation was not used to justify a grade,
and the evaluation in this chapter is only used to support
R3 (cost control), which was evaluated in @eval-cost-control, and
R5 (performance), which was evaluated in @eval-comparison.
Second, where the survey had already resolved a borderline case for another tool, the same resolution is applied to `petrivet`.
Third, where a requirement is met only by some parts of the API, the gaps are named.

*R1, programmatic access: ✓.*
`petrivet` is a library with a documented API:
every public item carries documentation, rendered by `rustdoc`,
and the crate documentation leads with a runnable example.
Each analysis is a method call with a typed result rather than a string or integer code:

#figure(
  [
    ```
    let mutual_exclusion_violation = petri_net
        .coverability()
        .target([(critical_section_a, 1), (critical_section_b, 1)])
        .call();
    match mutual_exclusion_violation {
        Some(Coverability::Coverable { marking, firing_sequence }) => { ... }
        Some(Coverability::Uncoverable { contradiction }) => { ... }
        None => { ... }
    }
    ```
  ],
  caption: [
    Example of a programmatic query for coverability,
    with a typed result that carries different evidence depending on the answer.
  ],
)

The API is only available from Rust.
Other languages need a wrapper:
either a command-line interface with structured output,
like the one built for the evaluation (`petrivet-cli`, which prints one JSON object per query),
or bindings produced by a generator such as
pyo3 #footnote[https://github.com/pyo3/pyo3] for Python,
cbindgen #footnote[https://github.com/mozilla/cbindgen] for C/C++,
ts-rs #footnote[https://github.com/aleph-alpha/ts-rs] for TypeScript, or
wasm-bindgen #footnote[https://github.com/wasm-bindgen/wasm-bindgen] for WebAssembly,
among others.
`petrivet` itself currently ships no such bindings.

*R2, safe portability: (✓).*
`petrivet` is written in Rust and forbids unsafe code in its own source (`#![forbid(unsafe_code)]` in `lib.rs`).
Its default configuration, however, links the Z3 SMT solver, which is written in C++.
By the rubric, this is a native-code dependency, and the survey graded the Z3-based checkers SMPT and Petrinizer "(✓)" for the
same reason.
Built with the pure-Rust solver OxiZ instead (`--no-default-features --features oxiz,pnml`),
`petrivet` has no native dependency left and would meet the requirement fully.

*R3, cost control: ✓.*
Every analysis method accepts a deadline,
and the methods for boundedness, liveness, and deadlock-freedom also accept an
effort restriction to polynomial procedures only:

#figure(
  [
    ```
    petri_net
        .is_deadlock_free()
        .effort(Effort::Efficient)
        .timeout(Duration::from_millis(100))
        .call()
    ```
  ],
  caption: "Example of a cost-controlled query for deadlock-freedom.",
)

A query that is not decided within these bounds returns None,
which the type system keeps distinct from a negative verdict.
@eval-cost-control shows that the restriction to polynomial procedures returns verdicts where brute force times out,
and that the deadline is kept within 200 ms in the large majority of runs;
it also names the stages that do not check the deadline and cause the tail.
There is no way to cancel a running query other than its deadline, which the rubric does not require.

*R4, model interchange: ✓.*
petrivet reads and writes PNML for place/transition nets
(`PetriNet::from_pnml`, `PetriNet::to_pnml`), behind the default feature `pnml`.
However, it only accepts ordinary, connected nets.
When the MCC models for the evaluation were drawn,
44 of the 134 drawn small models (33%) were rejected for this reason,
22 because of arc weights and 22 because they are not connected (corpus/mcc/selected.json).
The rubric grades reading PNML at all, so the grade is unaffected,
but in practice a third of the MCC models cannot be loaded.

*R5, adequate performance and memory: ✓.*
`petrivet` participated in the Model Checking Contest 2026 @kordon_complete_2026,
which, by the rubric, is independent evidence on non-trivial models;
like for the other tools in the surveyed, the grade reflects that this evidence exists, not how good it is.
The result was poor: `petrivet` placed last in every examination it entered.
The submitted build dates from May 2026 and predates most of the procedures described in @implementation,
so the contest measured an early version of the library and says little about the one evaluated in this thesis.
The current version is evaluated against LoLA, TINA, and ITS-Tools in @eval-comparison,
but that evaluation was conducted by the author of the tool, and its models and limits were chosen by him.
It shows `petrivet` competitive on liveness and on the parametrized families,
but behind LoLA and TINA on deadlock-freedom and quasi-liveness on the small MCC models.
Independent evidence for the current version would require participation in a future contest.

*R6, evidence with answers: ✓ (method, witness, certificate, theorem, culprits).*
Reachability and coverability return a firing sequence as a witness for a positive answer,
and for a negative answer the lemmas that contradict the target marking as a certificate.
Each lemma can be checked against the net on its own,
and together they form an infeasible linear system that can be re-checked by any solver,
which is a certificate in the sense of the rubric.
The deadlock iterator deadlocks() returns the reachable deadlock markings themselves,
though without the firing sequence leading to them.
The Commoner/Hack criterion returns the offending siphon and its unmarked maximal trap as culprits.
The documentation names the theorem behind each procedure and links to a literature index (petrivet::literature).
The evidence is uneven, however:
`liveness()` and `boundedness()` return only the verdict.
The procedure that decided them is recorded internally, but not yet exposed.

*R7, extensibility: ✓.*
The source is published under the LGPL 3.0, an OSI-approved license, and `petrivet` is usable as a library,
which meets the rubric.
Internally, procedures are written against generic interfaces where alternatives exist:
the CEGAR engine against the `SmtSolver` trait, and the state space explorer against the `TokenOps` trait of its marking type.
An extension concept for new model features, such as arc weights or time,
does not exist yet and is listed in the roadmap of the README.

*R8, approachability: (✓).*
Of the five repository indicators, `petrivet` meets four:
- a public GitHub repository with an issue tracker,
- a build from a clean checkout with the standard Rust toolchain (cargo build),
- automated tests (173 unit and integration tests, and a benchmark workflow in continuous integration),
- a `README.md` with a roadmap, and a `CONTRIBUTING.md` with contribution guidelines and a checklist for adding a new procedure,

With only one author and maintainer, the rubric's requirement for a team is not met.
Like the other tools maintained by a single person, `petrivet` therefore partly meets the requirement.

*Conclusion.*
`petrivet` fully meets six of the eight requirements (R1, R3, R4, R5, R6, R7), partly meets the other two (R2, R8),
and fails none, which is more than any tool in the survey (at most five).
The requirements that are not fully met are due to the C++ dependency of Z3 (R2) and the single maintainer (R8),
and not to any deficiency in the design or implementation of `petrivet` itself.
