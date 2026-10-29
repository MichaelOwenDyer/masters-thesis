#!/usr/bin/env python3
"""Draw one example net of each collection of the evaluation (calibration, randomized, small and large MCC models).

    python3 tools/make_collection_figures.py [-o figures/nets/collections]

The examples are fixed by name below. The calibration net and the randomized net are laid out automatically; the MCC
models are drawn with the positions stored in their PNML, scaled so that neighbouring nodes are about 40 px apart.
"""

import argparse
import math
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from petrinet_svg import Net, parse_pnml, render  # noqa: E402

CALIBRATION = "evaluation/calibration/cal-22-ac-mutex.pnml"
RANDOMIZED = "evaluation/corpus/gen/78585dd1e2048d28a5cd2512b4ec20c701e9d7ae39d621007c6fef900c2461c7.pnml"
MCC_SMALL = "evaluation/corpus/mcc/models/HouseConstruction-PT-00002/model.pnml"
MCC_LARGE = "evaluation/corpus/mcc/models/SquareGrid-PT-020102/model.pnml"


def rescale(net: Net, spacing: float = 40.0) -> Net:
    """Scale the stored positions so that the median distance of a node to its nearest neighbour is ``spacing``."""
    nodes = [*net.places.values(), *net.transitions.values()]
    ps = [d["pos"] for d in nodes]
    nearest = sorted(min(math.dist(a, b) for j, b in enumerate(ps) if j != i) for i, a in enumerate(ps))
    f = spacing / nearest[len(nearest) // 2]
    for d in nodes:
        d["pos"] = (d["pos"][0] * f, d["pos"][1] * f)
    return net


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", default="figures/nets/collections")
    a = ap.parse_args()
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)

    def read(path: str) -> Net:
        return parse_pnml(pathlib.Path(path).read_text(encoding="utf-8"))

    cal = read(CALIBRATION)
    labels = {n: f"{n[0]}_{n[1:]}" for n in [*cal.places, *cal.transitions]}  # p0 is drawn as p with subscript 0
    svgs = {
        "calibration": render(cal, label_map=labels, scale=1.3),
        "randomized": render(read(RANDOMIZED), scale=1.3),
        "mcc_small": render(rescale(read(MCC_SMALL))),
        "mcc_large": render(rescale(read(MCC_LARGE))),
    }
    for name, svg in svgs.items():
        (out / f"{name}.svg").write_text(svg, encoding="utf-8")
    print(f"wrote {len(svgs)} SVGs to {out}/")


if __name__ == "__main__":
    main()
