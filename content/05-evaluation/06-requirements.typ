== Assessment against the remaining requirements <eval-requirements>

Of the requirements R1-R8 that were established in @motivation,
R3 (cost control) was evaluated in @eval-cost-control,
and R5 (performance) was evaluated in @eval-comparison.
R1 (programmatic access),
R2 (portability and memory safety),
R4 (read/write standard model formats),
R6 (return evidence with verdicts),
R7 (extensibility), and
R8 (approachability)
are assessed here by inspection and demonstration,
by the same rubric that was applied to existing tools in @related-work.

*Programmatic access (R1).*
`petrivet` is first and foremost a library, designed to be used by other code.
It is written in Rust, and it exposes a public API that can be used from any Rust program.
It is also usable from other languages, either by wrapping it in a simple Rust CLI application (as in the evaluation),
or by using one of the many binding generators to produce bindings for the target language of choice:
pyo3 #footnote[https://github.com/pyo3/pyo3] for Python,
rust-bindgen #footnote[https://github.com/rust-lang/rust-bindgen] for C/C++,
ts-rs #footnote[https://github.com/aleph-alpha/ts-rs] for TypeScript, and
wasm-bindgen #footnote[https://github.com/wasm-bindgen/wasm-bindgen] for WebAssembly,
among others.