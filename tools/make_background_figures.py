#!/usr/bin/env python3
"""Draw the small example nets of the background section (structure, firing, behavioral and structural properties).

    python3 tools/make_background_figures.py [-o figures/nets/background]

Every net is written as one SVG named after it. The nets are tiny and placed by hand (positions in grid units, as in a
layout hint), so their pictures do not depend on the automatic layout. They are drawn at 1.3 times the usual size,
which puts the node labels at about the size of the body text.
"""

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from petrinet_svg import Net, render  # noqa: E402


def net(places: dict[str, tuple], transitions: dict[str, tuple], arcs: str) -> tuple[Net, dict]:
    """A net and its layout hint. ``places`` maps an id to (col, row) or (col, row, tokens), ``transitions`` maps an id
    to (col, row), and ``arcs`` lists the arcs as "a>b" separated by spaces."""
    n = Net()
    pos = {}
    for p, (c, r, *m) in places.items():
        n.places[p] = {"name": None, "marking": m[0] if m else 0, "pos": None}
        pos[p] = [c, r]
    for t, (c, r) in transitions.items():
        n.transitions[t] = {"name": None, "pos": None}
        pos[t] = [c, r]
    n.arcs = [(*a.split(">"), 1) for a in arcs.split()]
    return n, {"positions": pos}


# name -> (net and hint, draw the node ids, node sets to highlight)
FIGURES = {
    # structure: one place, one transition, one arc
    "structure": (net({"P1": (0, 0)}, {"T1": (1, 0)}, "P1>T1"), True, []),
    # a single marked place
    "token": (net({"P1": (0, 0, 1)}, {}, ""), False, []),
    # firing T1 consumes one token from each of P1 and P2 and produces one in P3
    "firing_before": (net({"P1": (0, -0.6, 2), "P2": (0, 0.6, 1), "P3": (2, 0)}, {"T1": (1, 0)},
                          "P1>T1 P2>T1 T1>P3"), True, []),
    "firing_after": (net({"P1": (0, -0.6, 1), "P2": (0, 0.6), "P3": (2, 0, 1)}, {"T1": (1, 0)},
                         "P1>T1 P2>T1 T1>P3"), True, []),
    # T1 has no input place, so it is always enabled and P1 grows without bound
    "unbounded": (net({"P1": (1, 0)}, {"T1": (0, 0)}, "T1>P1"), True, []),
    # trap {P1, P2}: unmarked at first, marked for good once T3 has fired
    "trap": (net({"P3": (0, 0, 1), "P1": (2, 0), "P2": (4, 0)}, {"T3": (1, 0), "T1": (3, -1), "T2": (3, 1)},
                 "P3>T3 T3>P1 P1>T1 T1>P2 P2>T2 T2>P1"), True, [["P1", "T1", "P2", "T2"]]),
    # siphon {P1, P2}: marked at first, empty for good once T3 has fired
    "siphon": (net({"P1": (0, 0, 1), "P2": (2, 0), "P3": (4, 0)}, {"T1": (1, -1), "T2": (1, 1), "T3": (3, 0)},
                   "P1>T1 T1>P2 P2>T2 T2>P1 P2>T3 T3>P3"), True, [["P1", "T1", "P2", "T2"]]),
    # structural subclasses
    "circuit": (net({"P1": (0, 0), "P2": (2, 0)}, {"T1": (1, -1), "T2": (1, 1)}, "P1>T1 T1>P2 P2>T2 T2>P1"), False, []),
    "sm_block": (net({"P1": (0, 0), "P2": (2, 0)}, {"T1": (1, 0)}, "P1>T1 T1>P2"), False, []),
    "sm": (net({"P1": (0, 0), "P2": (2, 0)}, {"T1": (1, -1), "T2": (1, 0), "T3": (1, 1)},
               "P1>T1 T1>P2 P1>T2 T2>P2 P2>T3 T3>P1"), False, []),
    "mg_block": (net({"P1": (1, 0)}, {"T1": (0, 0), "T2": (2, 0)}, "T1>P1 P1>T2"), False, []),
    "mg": (net({"P1": (1, -1), "P2": (1, 0), "P3": (1, 1)}, {"T1": (0, 0), "T2": (2, 0)},
               "T1>P1 P1>T2 T1>P2 P2>T2 T2>P3 P3>T1"), False, []),
    # P1 is a choice (not a marked graph) and T1 a fork (not a state machine)
    "fc": (net({"P1": (0, 0), "P2": (2, -1.2), "P3": (2, 0), "P4": (2, 1.2)}, {"T1": (1, -0.6), "T2": (1, 0.6)},
               "P1>T1 P1>T2 T1>P2 T1>P3 T2>P4"), False, []),
    # T1 and T2 share both of their input places
    "efc": (net({"P1": (0, -0.6), "P2": (0, 0.6), "P3": (2, -0.6), "P4": (2, 0.6)}, {"T1": (1, -0.6), "T2": (1, 0.6)},
                "P1>T1 P2>T1 P1>T2 P2>T2 T1>P3 T2>P4"), False, []),
    # the preset of T1 is a proper subset of the preset of T2
    "ac": (net({"P1": (0, -0.6), "P2": (0, 0.6), "P3": (2, -0.6), "P4": (2, 0.6)}, {"T1": (1, -0.6), "T2": (1, 0.6)},
               "P1>T1 P1>T2 P2>T2 T1>P3 T2>P4"), False, []),
    # T1 and T2 share P2, and neither preset contains the other
    "general": (net({"P1": (0, -1.2), "P2": (0, 0), "P3": (0, 1.2), "P4": (2, -0.6), "P5": (2, 0.6)},
                    {"T1": (1, -0.6), "T2": (1, 0.6)}, "P1>T1 P2>T1 P2>T2 P3>T2 T1>P4 T2>P5"), False, []),
}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", default="figures/nets/background")
    a = ap.parse_args()
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    for name, ((n, hint), ids, highlights) in FIGURES.items():
        labels = {v: f"{v[0]}_{v[1:]}" for v in [*n.places, *n.transitions]} if ids else None  # P1 is drawn as P with subscript 1
        svg = render(n, hint, label_map=labels, scale=1.3, highlights=highlights)
        (out / f"{name}.svg").write_text(svg, encoding="utf-8")
    print(f"wrote {len(FIGURES)} SVGs to {out}/")


if __name__ == "__main__":
    main()
