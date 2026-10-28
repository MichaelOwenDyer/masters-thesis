#!/usr/bin/env python3
"""Draw the small example nets of the background section (structure, firing, behavioral and structural properties).

    python3 tools/make_background_figures.py [-o figures/nets/background]

Every net is written as one SVG named after it, and so is the Venn diagram of the structural classes (classes.svg). The nets are tiny and placed by hand (positions in grid units, as in a
layout hint), so their pictures do not depend on the automatic layout. They are drawn at 1.3 times the usual size,
which puts the node labels at about the size of the body text.
"""

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from petrinet_svg import FONT, Net, render  # noqa: E402


def net(places: dict[str, tuple], transitions: dict[str, tuple], arcs: str, label_side: dict | None = None) -> tuple[Net, dict]:
    """A net and its layout hint. ``places`` maps an id to (col, row), (col, row, tokens) or (col, row, tokens, tokens of
    a target marking), ``transitions`` maps an id to (col, row), and ``arcs`` lists the arcs as "a>b" separated by spaces.
    ``label_side`` moves single labels away from arcs, as in a layout hint."""
    n = Net()
    pos = {}
    for p, (c, r, *m) in places.items():
        n.places[p] = {"name": None, "marking": m[0] if m else 0, "target": m[1] if len(m) > 1 else 0, "pos": None}
        pos[p] = [c, r]
    for t, (c, r) in transitions.items():
        n.transitions[t] = {"name": None, "pos": None}
        pos[t] = [c, r]
    n.arcs = [(*a.split(">"), 1) for a in arcs.split()]
    return n, {"positions": pos, "label_side": label_side or {}}


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
    # the firing example again, with a target marking (outlined) that is reachable by firing T1 once, and one that is not
    # (T1 would have to fire twice, but P2 has only one token)
    "reach_yes": (net({"P1": (0, -0.6, 2, 1), "P2": (0, 0.6, 1), "P3": (2, 0, 0, 1)}, {"T1": (1, 0)},
                      "P1>T1 P2>T1 T1>P3"), True, []),
    "reach_no": (net({"P1": (0, -0.6, 2), "P2": (0, 0.6, 1), "P3": (2, 0, 0, 2)}, {"T1": (1, 0)},
                     "P1>T1 P2>T1 T1>P3"), True, []),
    # T1 has no input place, so it is always enabled and P1 grows without bound
    "unbounded": (net({"P1": (1, 0)}, {"T1": (0, 0)}, "T1>P1"), True, []),
    # trap {P1, P2}: unmarked at first, marked for good once T3 has fired
    "trap": (net({"P3": (0, 0, 1), "P1": (2, 0), "P2": (4, 0)}, {"T3": (1, 0), "T1": (3, -1), "T2": (3, 1)},
                 "P3>T3 T3>P1 P1>T1 T1>P2 P2>T2 T2>P1"), True, [["P1", "T1", "P2", "T2"]]),
    # siphon {P1, P2}: marked at first, empty for good once T3 has fired
    "siphon": (net({"P1": (0, 0, 1), "P2": (2, 0), "P3": (4, 0)}, {"T1": (1, -1), "T2": (1, 1), "T3": (3, 0)},
                   "P1>T1 T1>P2 P2>T2 T2>P1 P2>T3 T3>P3"), True, [["P1", "T1", "P2", "T2"]]),
    # siphon {P1, P2} (light) contains the trap {P2} (dark); with the trap marked the siphon can never empty, with only P1
    # marked T3 can empty it
    "chc_yes": (net({"P2": (0, 0, 1), "P1": (2, 0), "P3": (4, 0)}, {"T1": (1, -1), "T2": (1, 1), "T3": (3, 0)},
                    "P1>T3 T3>P3 P1>T1 T1>P2 P2>T2 T2>P2 T2>P1", {"P2": "left"}), True,
                [{"nodes": ["P1", "T1", "P2", "T2"], "pad": 10}, {"nodes": ["P2"], "pad": 3, "fill": "#c8c8c8"}]),
    "chc_no": (net({"P2": (0, 0), "P1": (2, 0, 1), "P3": (4, 0)}, {"T1": (1, -1), "T2": (1, 1), "T3": (3, 0)},
                   "P1>T3 T3>P3 P1>T1 T1>P2 P2>T2 T2>P2 T2>P1", {"P2": "left"}), True,
               [{"nodes": ["P1", "T1", "P2", "T2"], "pad": 10}, {"nodes": ["P2"], "pad": 3, "fill": "#c8c8c8"}]),
    # one transition of each liveness level: T0 is dead, T1 can fire once, T2 as often as T3 has fired before T1,
    # T3 any number of times until T1 fires, and T4 always again (P3 is marked for good once T1 has fired)
    "liveness": (net({"P0": (0, 0), "P1": (2, 0, 1), "P2": (3, -1), "P3": (4, 0)},
                     {"T0": (1, 0), "T1": (3, 0), "T2": (4, -1), "T3": (2, -1), "T4": (5, 0)},
                     "P0>T0 T0>P1 P1>T3 T3>P1 T3>P2 P1>T1 T1>P3 P2>T2 P3>T2 T2>P3 P3>T4 T4>P3",
                     {"T2": "above", "T3": "above", "P2": "above"}), True, []),
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


def classes_venn(scale: float = 1.3) -> str:
    """Venn diagram of the structural classes: nested boxes from general nets down to free-choice nets, and inside the
    free-choice box the overlapping state machines and marked graphs, whose intersection is the circuits."""
    W, H = 320.0, 206.0
    out = []

    def text(x, y, s, anchor="start"):
        out.append(f'<text x="{x:.1f}" y="{y:.1f}" font-size="9" text-anchor="{anchor}" font-family="{FONT}">{s}</text>')

    # outermost first; each box is inset by 10 at the sides and bottom and by 20 at the top, to leave room for its name
    for i, (name, fill) in enumerate((("General nets", "#fff"), ("Asymmetric choice nets", "#f4f4f4"),
                                      ("Extended free-choice nets", "#e9e9e9"), ("Free-choice nets", "#dedede"))):
        x0, y0 = 1 + 10 * i, 1 + 20 * i
        out.append(f'<rect x="{x0}" y="{y0}" width="{W - 2 * x0:.1f}" height="{H - y0 - 1 - 10 * i:.1f}" rx="8" '
                   f'fill="{fill}" stroke="#000" stroke-width="0.8"/>')
        text(x0 + 7, y0 + 13, name)
    cy, rx, ry, d = 124.0, 72.0, 38.0, 58.0  # the two ellipses, d apart
    for paint in ('fill="#fff" fill-opacity="0.55"', 'fill="none" stroke="#000" stroke-width="0.8"'):  # outlines over both fills
        for cx in (W / 2 - d / 2, W / 2 + d / 2):
            out.append(f'<ellipse cx="{cx:.1f}" cy="{cy}" rx="{rx}" ry="{ry}" {paint}/>')
    text(W / 2 - d / 2 - rx / 2 - 4, cy - 2, "State", "middle")
    text(W / 2 - d / 2 - rx / 2 - 4, cy + 9, "machines", "middle")
    text(W / 2 + d / 2 + rx / 2 + 4, cy - 2, "Marked", "middle")
    text(W / 2 + d / 2 + rx / 2 + 4, cy + 9, "graphs", "middle")
    text(W / 2, cy + 3, "Circuits", "middle")
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W * scale:.1f}" height="{H * scale:.1f}" '
            f'viewBox="0 0 {W} {H}">{"".join(out)}</svg>\n')


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
    (out / "classes.svg").write_text(classes_venn(), encoding="utf-8")
    print(f"wrote {len(FIGURES) + 1} SVGs to {out}/")


if __name__ == "__main__":
    main()
