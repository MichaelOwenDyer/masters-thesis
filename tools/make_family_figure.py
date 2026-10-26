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
from petrinet_svg import parse_pnml, render, render_grid  # noqa: E402

ORDER = ["circuit", "sm_", "mg_", "efc_", "ac_", "general_"]


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
        items.append((f"{fam} (n = {d['size_param']})", net))
    out = pathlib.Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(render_grid(items, cols=a.cols), encoding="utf-8")
    single = out.parent / "families"
    single.mkdir(exist_ok=True)
    for (cap, net), fam in zip(items, sorted(smallest, key=rank)):
        (single / f"{fam}.svg").write_text(render(net), encoding="utf-8")
    print(f"wrote {out} with {len(items)} families and one SVG per family in {single}/")


if __name__ == "__main__":
    main()
