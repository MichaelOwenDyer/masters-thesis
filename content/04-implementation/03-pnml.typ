== PNML Import and Export <impl-pnml>

To facilitate interoperability with other Petri net tools,
`petrivet` also provides functionality for importing and exporting Petri nets in the PNML format,
following the specification defined in the PNML standard #footnote[https://www.pnml.org/].

Because the PNML format is XML-based, `petrivet` uses the `quick-xml`
#footnote[https://docs.rs/quick-xml/latest/quick_xml/]
crate to parse and generate XML data,
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

A `PnmlDocument` can then be converted (via `NetBuilder`) into a `Net` structure with relatively little code, and vice versa.
Since PNML is a versatile format that can represent a wide variety of Petri net types
and `petrivet` currently only supports ordinary Petri nets,
the conversion process is fallible and returns an error if the PNML document specifies an unsupported Petri net type
or contains otherwise invalid data.

A high-level method `Net::from_pnml()` is provided as an abstraction over these steps and
returns a `Net` structure from a PNML XML string, and `Net::to_pnml()` provides the reverse operation.
Corresponding methods are also provided for the `PetriNet` structure, which additionally parses the initial marking
specified in the PNML document. The `PetriNet` structure will be discussed in the next section.
