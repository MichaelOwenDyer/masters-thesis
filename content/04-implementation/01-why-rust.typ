== Why Rust

`petrivet` is implemented as a Rust library (NFR2) @klabnik_rust_2023,
a systems programming language whose design addresses several of the requirements
in @motivation-requirements directly.
Rust compiles to native code without a garbage collector or runtime,
which places its performance in the same class as C and C++ (R5).
Unlike those languages, however, the Rust compiler's ownership and borrowing rules
guarantee freedom from use-after-free errors, data races, and other classes of memory errors
before the program ever runs (R2) (NFR1) @jung_rustbelt_2017.
`petrivet` forbids `unsafe` code entirely (`#![forbid(unsafe_code)]`),
so these guarantees apply to the entire codebase.
#footnote[
  They do not, however, extend to the C++ code of the underlying Z3 solver,
  which is linked by default;
  the pure-Rust solver `OxiZ` can be selected instead via a compile-time feature flag,
  but it is not yet as mature as Z3 and is not used in the evaluation.
]
Because Rust does not need a runtime, a Rust library can be embedded almost anywhere:
it can be linked into native applications through a C-compatible interface,
wrapped for languages such as Python or TypeScript,
compiled to WebAssembly,
or shipped as a single static command-line binary (R1, R2).

Beyond safety and speed, Rust's type system helps to make the library both correct and approachable (NFR1).
Algebraic data types and exhausive pattern matching let the API state the possible outcomes of a query precisely,
and force the caller to handle each of them:
an answer of `None` for "unknown" cannot be confused with `Some(false)`,
and evidence (such as a witness firing sequence or a set of contradicting lemmas)
is carried only in the variant of the result it belongs to (R6).
Newtypes for place and transition IDs (@impl-builder) prevent mixing up indices of different kinds,
and traits allow procedures to be written once over a generic interface,
such as the state space explorer over marking types (@fig-state-space-api)
or the CEGAR engine over SMT solver backends (R7).

Finally, Rust's tooling lowers the barrier for new contributors (R8):
Cargo builds, tests, benchmarks and documents the library with a single command each,
and the compiler's detailed error messages catch many mistakes that would otherwise only appear at runtime.
This is valuable for students and AI agents alike, who receive immediate and precise feedback when they extend the library.