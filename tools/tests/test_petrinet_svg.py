import json
import math
import pathlib
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, str(pathlib.Path(__file__).parents[1]))
import petrinet_svg as P  # noqa: E402

PNML = """<pnml><net id="n" type="x"><page id="pg">
<place id="p0"><initialMarking><text>2</text></initialMarking></place><place id="p1"/>
<transition id="t0"/><transition id="t1"/>
<arc id="a0" source="p0" target="t0"><inscription><text>3</text></inscription></arc>
<arc id="a1" source="t0" target="p1"/><arc id="a2" source="p1" target="t1"/><arc id="a3" source="t1" target="p0"/>
</page></net></pnml>"""


def test_parse_weights_and_marking():
    n = P.parse_pnml(PNML)
    assert n.places["p0"]["marking"] == 2 and set(n.transitions) == {"t0", "t1"}
    assert ("p0", "t0", 3) in n.arcs and ("t0", "p1", 1) in n.arcs


def test_rejects_place_to_place_arc():
    bad = PNML.replace('source="t0" target="p1"', 'source="p0" target="p1"')
    try:
        P.parse_pnml(bad)
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_valid_svg_with_all_nodes_and_arcs_and_deterministic():
    n = P.parse_pnml(PNML)
    svg = P.render(n)
    root = ET.fromstring(svg)
    tags = [e.tag.rsplit("}", 1)[-1] for e in root.iter()]
    assert tags.count("rect") == 2 and tags.count("path") - 1 == 4  # one path is the arrowhead
    assert svg == P.render(P.parse_pnml(PNML))
    assert ">3<" in svg  # weight label


def test_nodes_do_not_overlap():
    n = P.parse_pnml(PNML)
    pos, _ = P.layout(n)
    real = [p for k, p in pos.items() if not k.startswith("~")]
    for i, a in enumerate(real):
        for b in real[i + 1:]:
            assert math.dist(a, b) >= 2 * P.R_PLACE


def test_hint_positions_override():
    n = P.parse_pnml(PNML)
    pos, _ = P.layout(n, {"positions": {"p0": [0, 0], "t0": [2, 0], "p1": [4, 0], "t1": [6, 0]}})
    assert pos["t0"][0] - pos["p0"][0] == 2 * P.DX and pos["p0"][1] == pos["t1"][1]


def test_pnml_graphics_positions_used():
    g = PNML.replace('<place id="p0">', '<place id="p0"><graphics><position x="0" y="0"/></graphics>')
    for i, nid in enumerate(["p1", "t0", "t1"], 1):
        g = g.replace(f'<{"place" if nid[0]=="p" else "transition"} id="{nid}"/>',
                      f'<{"place" if nid[0]=="p" else "transition"} id="{nid}"><graphics><position x="{40*i}" y="5"/></graphics></{"place" if nid[0]=="p" else "transition"}>')
    pos, _ = P.layout(P.parse_pnml(g))
    assert pos["p1"] == (40, 5) and pos["t1"] == (120, 5)


def test_all_family_nets_render():
    man = pathlib.Path(__file__).parents[2] / "evaluation/corpus/fam/manifest.jsonl"
    if not man.exists():
        return
    seen = set()
    for line in man.read_text().splitlines():
        d = json.loads(line)
        if d["family"] in seen:
            continue
        seen.add(d["family"])
        net = P.parse_pnml((man.parent / d["path"]).read_text())
        ET.fromstring(P.render(net))
    assert len(seen) >= 10


def _buffer_ring(n):
    parts = [f'<transition id="t{i}"/>' for i in range(n)]
    arcs = []
    for i in range(n):
        j = (i + 1) % n
        parts += [f'<place id="r{i}"/>', f'<place id="b{i}"/>']
        arcs += [(f"t{i}", f"r{i}"), (f"r{i}", f"t{j}"), (f"t{j}", f"b{i}"), (f"b{i}", f"t{i}")]
    body = "".join(parts) + "".join(f'<arc id="a{k}" source="{a}" target="{b}"/>' for k, (a, b) in enumerate(arcs))
    return P.parse_pnml(f"<pnml><net id='n'><page id='p'>{body}</page></net></pnml>")


def test_buffer_ring_loops_keep_their_direction():
    n = 5
    net = _buffer_ring(n)
    pos, chains = P.layout(net)
    assert chains == {}
    cx = sum(pos[f"t{i}"][0] for i in range(n)) / n
    cy = sum(pos[f"t{i}"][1] for i in range(n)) / n
    rad = lambda v: math.dist(pos[v], (cx, cy))
    r_t = sum(rad(f"t{i}") for i in range(n)) / n
    sides = {(rad(f"r{i}") > r_t) for i in range(n)} | {(rad(f"b{i}") > r_t) for i in range(n)}
    assert sides == {True, False}
    # all forward places are on one side and all backward places on the other
    assert len({rad(f"r{i}") > r_t for i in range(n)}) == 1 and len({rad(f"b{i}") > r_t for i in range(n)}) == 1
