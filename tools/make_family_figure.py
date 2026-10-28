#!/usr/bin/env python3
"""Draw the smallest instance of every parametrized family of the scaling corpus as one panel grid.

    python3 tools/make_family_figure.py [--manifest evaluation/corpus/fam/manifest.jsonl] [-o figures/nets/families.svg]
                                       [--cols 3]

Families are ordered by the structural class they are meant to exercise (the order of ``ORDER``); the caption of each
panel is the family name (as in the corpus, with underscores) and the size parameter of the drawn instance. Nets are read from the PNML files next to the
manifest, so the figure follows the corpus.
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from petrinet_svg import DX, DY, parse_pnml, render, render_grid  # noqa: E402

# families drawn with the spring layout turned upright (tall pictures suit the cells of the family table)
TALL = {"ac_cycle", "ac_cycle_no_low_token", "general_cycle"}
# families drawn with a spring layout turned to the most square-shaped picture
SQUARE = {"circuit", "sm_hub_and_spokes"}


def choice_ring_hint(n: int) -> dict:
    """sm_choice_ring: the entry places of the n stages sit on a circle; between two of them the two parallel branches
    (choose, middle place, commit) run side by side along the circle, the first on the inner and the second on the outer track.
    Node ids as in the generator: entry p_i = place i, middle place of stage i and branch c = place n + 2i + c,
    choose = transition 4i + 2c, commit = transition 4i + 2c + 1."""
    import math
    r, d = 78.0, 14.0
    pos: dict[str, list[float]] = {}

    def put(node: str, phi: float, radius: float) -> None:
        pos[node] = [radius * math.sin(phi) / DX, -radius * math.cos(phi) / DY]

    for i in range(n):
        a0, a1 = 2 * math.pi * i / n, 2 * math.pi * (i + 1) / n
        put(f"p{i}", a0, r)
        for c in range(2):
            radius = r - d if c == 0 else r + d
            for k, node in enumerate((f"t{4 * i + 2 * c}", f"p{n + 2 * i + c}", f"t{4 * i + 2 * c + 1}"), 1):
                put(node, a0 + (a1 - a0) * k / 4, radius)
    return {"positions": pos}


def efc_ring_hint(n: int) -> dict:
    """efc_forkjoin and efc_forkjoin_leaky: the entry places of the n stages sit on a circle. Between two of them the stage
    runs along the circle in three steps: the two choices u and v (inner and outer track), the two places x and y they
    fill (inner and outer track) and the join w on the middle track.
    Node ids as in the generator: entry p_i = place i, x_i = place n + 2i, y_i = place n + 2i + 1,
    u_i = transition 3i, v_i = transition 3i + 1, join w_i = transition 3i + 2."""
    import math
    r, d = 100.0, 22.0
    pos: dict[str, list[float]] = {}

    def put(node: str, phi: float, radius: float) -> None:
        pos[node] = [radius * math.sin(phi) / DX, -radius * math.cos(phi) / DY]

    for i in range(n):
        a0, a1 = 2 * math.pi * i / n, 2 * math.pi * (i + 1) / n
        put(f"p{i}", a0, r)
        for k, nodes in enumerate(((f"t{3 * i}", f"t{3 * i + 1}"), (f"p{n + 2 * i}", f"p{n + 2 * i + 1}")), 1):
            put(nodes[0], a0 + (a1 - a0) * k / 4, r - d)
            put(nodes[1], a0 + (a1 - a0) * k / 4, r + d)
        put(f"t{3 * i + 2}", a0 + (a1 - a0) * 3 / 4, r)
    return {"positions": pos}


def two_cycles_hint(n: int) -> dict:
    """sm_two_cycles and sm_two_cycles_n_tokens: the cycle A (with the tokens) on top of the cycle B, each a circle of its
    places and transitions in alternation, joined by the one-way switch between the bottom of A (A_0) and the top of B (B_0).
    Node ids as in the generator, with half = max(n / 2, 2): A_i = place i, B_i = place half + i, a_i = transition i,
    b_i = transition half + i (the transition after the place), switch = transition 2 * half."""
    import math
    half = max(n // 2, 2)
    r = max(34.0, 2 * half * 26.0 / (2 * math.pi))
    gap = 30.0  # distance of A_0 and B_0 from the switch
    pos: dict[str, list[float]] = {}
    for ring, (cy, first_place, first_trans, start) in enumerate(((-(r + gap), 0, 0, math.pi), (r + gap, half, half, 0.0))):
        for i in range(half):
            for k, node in enumerate((f"p{first_place + i}", f"t{first_trans + i}")):
                phi = start + 2 * math.pi * (2 * i + k) / (2 * half)  # clockwise from the starting point
                pos[node] = [r * math.sin(phi) / DX, (cy - r * math.cos(phi)) / DY]
    pos[f"t{2 * half}"] = [0.0, 0.0]
    return {"positions": pos}


HINTS = {"sm_two_cycles": two_cycles_hint, "sm_two_cycles_n_tokens": two_cycles_hint, "sm_choice_ring": choice_ring_hint, "efc_forkjoin": efc_ring_hint, "efc_forkjoin_leaky": efc_ring_hint}
ORDER = ["circuit", "sm_", "mg_", "efc_", "ac_", "general_"]


def display_name(fam: str) -> str:
    """The family id as it is written in the thesis: hyphens, and fc for the two free-choice families."""
    return fam.replace("_", "-").replace("efc-", "fc-", 1)


def rank(fam: str) -> tuple[int, str]:
    for i, p in enumerate(ORDER):
        if fam.startswith(p):
            return i, fam
    return len(ORDER), fam


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", default="evaluation/corpus/fam/manifest.jsonl")
    ap.add_argument("-o", "--out", default="figures/nets/families.svg")
    ap.add_argument("--cols", type=int, default=3)
    a = ap.parse_args()
    man = pathlib.Path(a.manifest)
    smallest: dict[str, dict] = {}
    for line in man.read_text().splitlines():
        d = json.loads(line)
        cur = smallest.get(d["family"])
        if cur is None or (d["size_param"], d["path"]) < (cur["size_param"], cur["path"]):
            smallest[d["family"]] = d
    items = []
    for fam in sorted(smallest, key=rank):
        d = smallest[fam]
        net = parse_pnml((man.parent / d["path"]).read_text(encoding="utf-8"))
        hint = HINTS[fam](d["size_param"]) if fam in HINTS else None
        items.append((f"{display_name(fam)} (n = {d['size_param']})", net, hint))
    out = pathlib.Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(render_grid(items, cols=a.cols), encoding="utf-8")
    single = out.parent / "families"
    single.mkdir(exist_ok=True)
    for (cap, net, hint), fam in zip(items, sorted(smallest, key=rank)):
        svg = (render(net, hint) if hint else render(net, method="force", tall=True) if fam in TALL
               else render(net, method="force", square=True) if fam in SQUARE else render(net))
        (single / f"{fam}.svg").write_text(svg, encoding="utf-8")
    print(f"wrote {out} with {len(items)} families and one SVG per family in {single}/")


if __name__ == "__main__":
    main()
