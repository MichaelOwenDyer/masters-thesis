#import "../lib.typ": *

#let small-table(columns, align, header, ..cells) = text(size: 8pt, table(
  columns: columns, align: align, inset: 3pt, stroke: 0.4pt + luma(170),
  table.header(..header.map(h => [*#h*])), ..cells,
))

== External Tool Adapters <eval-adapters>

This section details the implementation of the adapters for the external tools described in @eval-tools.
Each adapter takes an evaluation query and translates it into one command line for the tool,
and translates the tool's output into a verdict (true, false, a count, or "unknown").
Where the tool does not support a property directly,
the adapter runs several calls of the tool as one shell script, and the time and memory of the whole script are measured.

*Input.*
TINA and ITS-Tools read PNML directly
(ITS-Tools expects a folder containing a file `model.pnml`, which the adapter creates for each run and removes afterwards).
LoLA uses its own proprietary format, but is distributed with a tool `ndrio` that converts PNML to LoLA format,
preserving the PNML identifiers of places and transitions.

*Properties.*
@tab-adapters shows how each property is obtained from each tool.
A property is answered _directly_ when the tool has a command for exactly that property,
by a _formula_ when it is expressed in the tool's query language and answered by one call,
and by a _composition_ when the adapter combines several calls.

#figure(
  kind: table,
  caption: [
    How the adapters obtain each property. $p_1, dots, p_n$ are the places with initial marking $m_1, dots, m_n$,
    $t$ ranges over the transitions, and $a_i$ is the target marking of $p_i$.
  ],
  small-table(
    (auto, 1fr, 1fr, 1fr), (left, left, left, left),
    ("Property", "LoLA 2.0", "TINA 4.0.0", "ITS-Tools"),
    [Deadlock-free],
    [formula `EF DEADLOCK`, negated],
    [direct: reachability graph (`tina -R`), true if it has no dead marking],
    [direct: examination `ReachabilityDeadlock`, negated],
    [Bounded],
    [formula on the coverability graph (`--search=cover`): `IMPOSSIBLE` $(p_1 = omega or dots or p_n = omega)$],
    [direct: the boundedness line of `tina -R`],
    [not supported],
    [1-safe],
    [formula `AG` $(p_1 <= 1 and dots and p_n <= 1)$],
    [direct: `sift` with the predefined condition `safe`],
    [direct: examination `OneSafe`],
    [Live],
    [*composition*: `AGEF FIREABLE(`$t$`)` for each transition, stopping at the first false],
    [direct: the liveness line of `tina -R` (bounded nets only)],
    [not supported],
    [Quasi-live],
    [*composition*: `EF FIREABLE(`$t$`)` for each transition, stopping at the first false],
    [direct: `tina -R`, true if it has no dead transition (bounded nets only)],
    [direct: examination `QuasiLiveness`],
    [Reachable],
    [formula `EF` $(p_1 = a_1 and dots and p_n = a_n)$ over all places],
    [*composition*: state space (`sift`), then the same formula with `scan`],
    [not supported],
    [Coverable],
    [formula `EF` $(and.big_(a_i > 0) p_i >= a_i)$],
    [*composition*: state space (`sift`), then the same formula with `scan`],
    [not supported],
    [State space size],
    [direct: full state space (`--check=full`, partial-order reduction off)],
    [direct: the marking and transition counts of `tina -R` (bounded nets only)],
    [direct: examination `StateSpace` (markings only)],
    [Stable place],
    [*composition*: `AG` $(p = m)$ for each place, stopping at the first true],
    [*composition*: state space (`sift`), then for each place `scan` checks that no state violates $p = m$, stopping at the first true],
    [direct: examination `StableMarking`],
  ),
) <tab-adapters>

*Compositions.*
A composition executes several calls in a fixed order and short-circuits as soon as a verdict becomes known.
If every call finishes and the property is still unknown, the composition returns "unknown".
If a call fails, the composition returns the error message of the failing call.
For liveness, the adapter for LoLA runs the CTL formula `AGEF FIREABLE(t)` for each transition $t$ in the net, stopping at the first false.
A single formula over all transitions was also attempted, but this caused the tool to exceed the memory limit on
calibration nets with only six transitions.
Quasi-liveness is composed of `EF FIREABLE(t)` for each transition $t$.
Stable place is composed of `AG (p = m)` for each place $p$ with initial marking $m$.
TINA's `scan` evaluates formulas on a state space previously cached by `sift`,
so stable-place, reachability and coverability are composed of a `sift` call followed by one or more `scan` calls.
Reachability queries are encoded as a CTL formula over all places (including those with target marking 0), and
coverability queries are encoded as a CTL formula over all places with target marking greater than 0.

*Not supported.*
ITS-Tools is run through the examinations of the Model Checking Contest, but the contest has no examination for boundedness.
On liveness, ITS-Tools spends a fixed 30 seconds in a random walk on every net regardless of time-out setting,
which exceeds the time limit of the evaluation.
These properties are therefore not posed to ITS-Tools.

*Validation.*
Every adapter was tested on all calibration nets (@eval-collections), whose answers were derived by hand.
No verdict of LoLA or TINA contradicted the expected answer.
ITS-Tools reported one deadlock in a deadlock-free unbounded net,
the same error that the reference comparison finds on all unbounded nets (@eval-correctness).
