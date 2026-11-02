== PNML Import and Export <impl-pnml>

To facilitate interoperability with other Petri net tools,
`petrivet` also provides functionality for importing and exporting Petri nets in the PNML format,
following the specification defined in the PNML standard #footnote[https://www.pnml.org/].
PNML interchange is implemented behind the "pnml" feature flag (enabled by default)
which can be disabled if not needed to reduce the size of the compiled binary.

Because the PNML format is XML-based, `petrivet` uses the `quick-xml`
#footnote[https://docs.rs/quick-xml/latest/quick_xml/]
crate (Rust library) to parse and generate XML data,
which integrates with `serde` #footnote[https://docs.rs/serde/latest/serde/]
to serialize and deserialize the data structures.
These are two well-established crates in the Rust ecosystem,
and they significantly lower the boilerplate code required to implement PNML interchange.

`petrivet` defines a Rust data structure `PnmlDocument` corresponding to the PNML grammar
and annotates it with `serde` attributes,
then simply calls `quick-xml::de::from_str()` to deserialize a PNML XML string into a `PnmlDocument` structure (FR2, import),
and `quick-xml::se::to_string()` to serialize a `PnmlDocument` structure into a PNML XML string (FR3, export).
In the interest of low coupling, `petrivet` does not handle any kind of file or network I/O,
leaving that responsibility to the user of the library #footnote[https://sans-io.readthedocs.io/how-to-sans-io.html].

A `PnmlDocument` represents any valid PNML file, which can contain multiple Petri nets models,
and model types that are not supported by `petrivet` (e.g. colored Petri nets, timed Petri nets, etc.).
The conversion into native `Net` structures is therefore a separate fallible step.
A high-level method `Net::from_pnml()` is provided as an abstraction over these steps and
returns a `Net` structure from a PNML XML string (the first model in the file, if there are multiple),
and `Net::to_pnml()` provides the reverse operation.
Corresponding methods are also provided for the `PetriNet` structure, which additionally parses the initial marking
specified in the PNML document. The `PetriNet` structure will be discussed in the next section.

All IDs, names, and other metadata in the PNML file are preserved in the `PnmlDocument` structure
and stored read-only in the `Net` structure so that a round-trip conversion from PNML to `Net`
and back will yield the same document.
Note that `petrivet` currently does not support modifying this metadata or setting it when creating a new net.
This is a planned feature for future versions of `petrivet`.
