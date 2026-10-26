#!/usr/bin/env python3
"""Static Petri net renderer: PNML in, SVG out (standard library only).

    python3 tools/petrinet_svg.py net.pnml -o net.svg [--rankdir LR|TB] [--labels none|ids|names]
                                 [--label-map map.json] [--hint hint.json] [--scale 1.0]

Layout: if every node has a <graphics><position> in the PNML, those positions are used. Otherwise a layered layout
(Sugiyama style) is computed: cycles are broken by reversing DFS back edges, nodes are layered by longest path, long
edges get dummy nodes, and the order within each layer is improved by barycenter sweeps. Edges are smooth curves through
their dummy nodes, so cycles and long edges are routed around the nodes in between.

A hint file (JSON) overrides single nodes: {"positions": {"p0": [col, row], ...}} in grid units (1 unit = one node
distance); nodes not mentioned keep their computed place. With positions for every node no layering is done.

Drawing: places are circles, transitions are squares, arcs carry an arrowhead and a weight label when the weight is
not 1, tokens are dots (a number above four). The module can be imported: ``parse_pnml``, ``layout``, ``render``,
``render_grid`` (several nets as labelled panels in one SVG).
"""

from __future__ import annotations

import argparse
import random
import json
import math
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from html import escape

FONT = "TeX Gyre Termes, Times New Roman, serif"
R_PLACE = 9.0
S_TRANS = 14.0  # side length
DX, DY = 52.0, 38.0  # layer distance, node distance in a layer
MARGIN = 16.0


@dataclass
class Net:
    places: dict[str, dict] = field(default_factory=dict)  # id -> {name, marking, pos}
    transitions: dict[str, dict] = field(default_factory=dict)  # id -> {name, pos}
    arcs: list[tuple[str, str, int]] = field(default_factory=list)

    def is_place(self, n: str) -> bool:
        return n in self.places


def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _text(el: ET.Element, child: str) -> str | None:
    for c in el:
        if _local(c.tag) == child:
            for t in c.iter():
                if _local(t.tag) == "text" and t.text is not None:
                    return t.text.strip()
    return None


def _pos(el: ET.Element) -> tuple[float, float] | None:
    for c in el:
        if _local(c.tag) == "graphics":
            for p in c:
                if _local(p.tag) == "position":
                    try:
                        return float(p.get("x")), float(p.get("y"))
                    except (TypeError, ValueError):
                        return None
    return None


def parse_pnml(source: str) -> Net:
    """Parse the first net of a PNML document (place/transition nets; all pages are merged)."""
    root = ET.fromstring(source)
    net = Net()
    for el in root.iter():
        t = _local(el.tag)
        if t == "place":
            m = _text(el, "initialMarking")
            net.places[el.get("id")] = {"name": _text(el, "name"), "marking": int(m) if m else 0, "pos": _pos(el)}
        elif t == "transition":
            net.transitions[el.get("id")] = {"name": _text(el, "name"), "pos": _pos(el)}
    for el in root.iter():
        if _local(el.tag) == "arc":
            w = _text(el, "inscription")
            net.arcs.append((el.get("source"), el.get("target"), int(w) if w else 1))
    for s, t, _ in net.arcs:
        if s not in net.places and s not in net.transitions or t not in net.places and t not in net.transitions:
            raise ValueError(f"arc {s} -> {t} refers to an unknown node")
        if (s in net.places) == (t in net.places):
            raise ValueError(f"arc {s} -> {t} does not connect a place and a transition")
    return net


# ------------------------------------------------------------------ layout

def _layers(net: Net) -> tuple[dict[str, int], set[tuple[str, str]]]:
    """Longest-path layers after reversing DFS back edges. Returns (layer, reversed edge set)."""
    nodes = list(net.places) + list(net.transitions)
    succ: dict[str, list[str]] = {n: [] for n in nodes}
    for s, t, _ in net.arcs:
        if t not in succ[s]:
            succ[s].append(t)
    marked = [p for p in net.places if net.places[p]["marking"] > 0]
    indeg = {n: 0 for n in nodes}
    for s in succ:
        for t in succ[s]:
            indeg[t] += 1
    roots = marked + [n for n in nodes if indeg[n] == 0 and n not in marked] + nodes
    state: dict[str, int] = {}
    back: set[tuple[str, str]] = set()
    for r in roots:
        if r in state:
            continue
        stack = [(r, iter(succ[r]))]
        state[r] = 1
        while stack:
            n, it = stack[-1]
            for m in it:
                if state.get(m) == 1:
                    back.add((n, m))
                elif m not in state:
                    state[m] = 1
                    stack.append((m, iter(succ[m])))
                    break
            else:
                state[n] = 2
                stack.pop()
    dag = {n: [] for n in nodes}
    for s in succ:
        for t in succ[s]:
            if (s, t) in back:
                dag[t].append(s)
            else:
                dag[s].append(t)
    layer = {n: 0 for n in nodes}
    order, seen = [], set()

    def topo(n):  # iterative post-order
        stack = [(n, iter(dag[n]))]
        seen.add(n)
        while stack:
            u, it = stack[-1]
            for v in it:
                if v not in seen:
                    seen.add(v)
                    stack.append((v, iter(dag[v])))
                    break
            else:
                order.append(u)
                stack.pop()

    for n in nodes:
        if n not in seen:
            topo(n)
    for n in reversed(order):
        for m in dag[n]:
            layer[m] = max(layer[m], layer[n] + 1)
    # longest-path layering pushes nodes with few predecessors too far left, and their edges become long;
    # move each node within its free range to where it shortens its edges (more edges to the right: right).
    preds = {n: [] for n in nodes}
    for n in dag:
        for m in dag[n]:
            preds[m].append(n)
    for _ in range(6):
        changed = False
        for n in nodes:
            lo = max((layer[u] + 1 for u in preds[n]), default=None)
            hi = min((layer[v] - 1 for v in dag[n]), default=None)
            if lo is None and hi is None:
                continue
            a, b = len(preds[n]), len(dag[n])
            if lo is None:
                new = hi  # source: as late as possible
            elif hi is None:
                new = lo
            elif b > a:
                new = lo if lo > hi else lo
            elif a > b:
                new = hi
            else:
                continue
            if lo is not None and hi is not None:
                if hi < lo:
                    continue
                new = hi if a > b else lo
            if new != layer[n]:
                layer[n] = new
                changed = True
        if not changed:
            break
    base = min(layer.values(), default=0)
    layer = {n: v - base for n, v in layer.items()}
    return layer, back


def _polylines(net: Net, pos, chains) -> list[tuple[str, str, list]]:
    out = []
    for s, t, _ in net.arcs:
        ch = chains.get((s, t)) or (list(reversed(chains[(t, s)])) if (t, s) in chains else None)
        out.append((s, t, [pos[c] for c in ch] if ch and len(ch) > 2 else [pos[s], pos[t]]))
    return out


def _seg_cross(a, b, c, d) -> bool:
    def o(p, q, r):
        return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    return o(a, b, c) * o(a, b, d) < 0 and o(c, d, a) * o(c, d, b) < 0


def _score(net: Net, pos, chains) -> float:
    """Edge crossings plus edges passing through a node, plus a little for total edge length."""
    pl = _polylines(net, pos, chains)
    segs = [(s, t, p, q) for s, t, pts in pl for p, q in zip(pts, pts[1:])]
    cross = 0
    for i in range(len(segs)):
        for j in range(i + 1, len(segs)):
            a, b = segs[i], segs[j]
            if {a[0], a[1]} & {b[0], b[1]}:
                continue
            cross += _seg_cross(a[2], a[3], b[2], b[3])
    through = 0
    for s, t, p, q in segs:
        for n, c in pos.items():
            if n in (s, t) or n.startswith("~"):
                continue
            dx, dy = q[0] - p[0], q[1] - p[1]
            L = dx * dx + dy * dy or 1.0
            u = max(0.0, min(1.0, ((c[0] - p[0]) * dx + (c[1] - p[1]) * dy) / L))
            if math.hypot(p[0] + u * dx - c[0], p[1] + u * dy - c[1]) < 12:
                through += 1
    length = sum(math.hypot(q[0] - p[0], q[1] - p[1]) for _, _, p, q in segs)
    return 10 * cross + 10 * through + length / 1000


def _rotate(pos):
    """Rotate so that the longer extent (principal axis) of the drawing is horizontal."""
    xs = [p[0] for p in pos.values()]
    ys = [p[1] for p in pos.values()]
    mx, my = sum(xs) / len(xs), sum(ys) / len(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    syy = sum((y - my) ** 2 for y in ys)
    sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    th = 0.5 * math.atan2(2 * sxy, sxx - syy)
    c, s_ = math.cos(-th), math.sin(-th)
    return {n: ((x - mx) * c - (y - my) * s_, (x - mx) * s_ + (y - my) * c) for n, (x, y) in pos.items()}


def _force(net: Net, seed: int):
    """Fruchterman-Reingold from a deterministic start; edges are straight."""
    nodes = list(net.places) + list(net.transitions)
    rnd = random.Random(seed)
    n = len(nodes)
    k = 46.0
    pos = {v: [math.cos(2 * math.pi * i / n) * k * n / 6.3 + rnd.uniform(-5, 5),
               math.sin(2 * math.pi * i / n) * k * n / 6.3 + rnd.uniform(-5, 5)] for i, v in enumerate(nodes)}
    edges = sorted({(s, t) for s, t, _ in net.arcs})  # sorted: the layout must not depend on hash order
    temp = k * 2
    for _ in range(300):
        disp = {v: [0.0, 0.0] for v in nodes}
        for i, u in enumerate(nodes):
            for v in nodes[i + 1:]:
                dx, dy = pos[u][0] - pos[v][0], pos[u][1] - pos[v][1]
                d = math.hypot(dx, dy) or 0.01
                f = k * k / d
                disp[u][0] += dx / d * f; disp[u][1] += dy / d * f
                disp[v][0] -= dx / d * f; disp[v][1] -= dy / d * f
        for s, t in edges:
            dx, dy = pos[s][0] - pos[t][0], pos[s][1] - pos[t][1]
            d = math.hypot(dx, dy) or 0.01
            f = d * d / k
            disp[s][0] -= dx / d * f; disp[s][1] -= dy / d * f
            disp[t][0] += dx / d * f; disp[t][1] += dy / d * f
        for v in nodes:
            d = math.hypot(*disp[v]) or 0.01
            m = min(d, temp)
            pos[v][0] += disp[v][0] / d * m
            pos[v][1] += disp[v][1] / d * m
        temp *= 0.985
    return {v: (x, y) for v, (x, y) in pos.items()}


def _ring(net: Net):
    """Layout for a ring of transitions with one place per direction between neighbours (a buffer ring).

    The transitions sit on a circle. The places of the arcs that run counter-clockwise are on the outside and those
    of the arcs that run clockwise are on the inside, so each loop keeps its direction all the way round.
    Returns None for any other net.
    """
    ts = sorted(net.transitions)
    n = len(ts)
    if n < 3:
        return None
    ins: dict[str, list[str]] = {p: [] for p in net.places}
    outs: dict[str, list[str]] = {p: [] for p in net.places}
    for a, b, _ in net.arcs:
        if a in net.places:
            outs[a].append(b)
        else:
            ins[b].append(a)
    between: dict[tuple[str, str], list[str]] = {}
    for p in net.places:
        if len(ins[p]) != 1 or len(outs[p]) != 1 or ins[p][0] == outs[p][0]:
            return None
        between.setdefault((ins[p][0], outs[p][0]), []).append(p)
    nb: dict[str, set[str]] = {t: set() for t in ts}
    for (a, b) in between:
        nb[a].add(b)
        nb[b].add(a)
    if any(len(v) != 2 for v in nb.values()) or len(net.places) != 2 * n:
        return None
    if any(len(v) != 1 for v in between.values()) or len(between) != 2 * n:
        return None
    order = [ts[0]]
    while len(order) < n:
        nxt = [m for m in sorted(nb[order[-1]]) if m not in order]
        if not nxt:
            return None
        order.append(nxt[0])
    if order[0] not in nb[order[-1]]:
        return None
    spacing = 80.0
    R = max(60.0, spacing * n / (2 * math.pi))
    off = 22.0
    pos = {}
    for i, t in enumerate(order):
        a = 2 * math.pi * i / n
        pos[t] = (R * math.cos(a), -R * math.sin(a))
        u = order[(i + 1) % n]
        mid = a + math.pi / n
        ccw, cw = between[(t, u)][0], between[(u, t)][0]  # t -> u runs counter-clockwise
        pos[ccw] = ((R + off) * math.cos(mid), -(R + off) * math.sin(mid))
        pos[cw] = ((R - off) * math.cos(mid), -(R - off) * math.sin(mid))
    return pos


def layout(net: Net, hint: dict | None = None, rankdir: str = "LR", method: str = "auto"):
    """Return (positions in pixels of the nodes and of the dummy nodes of long edges, chains of dummy nodes per edge)."""
    nodes = list(net.places) + list(net.transitions)
    fixed = {n: net.places.get(n, net.transitions.get(n))["pos"] for n in nodes}
    chains: dict = {}
    if nodes and all(fixed[n] is not None for n in nodes):
        pos = {n: fixed[n] for n in nodes}
    else:
        ring = _ring(net) if method in ("auto", "ring") else None
        if ring is not None:
            pos, chains = ring, {}
        else:
            pos, chains = _sugiyama(net, rankdir)
        if ring is None and method != "layered":
            # a layered picture of a cyclic net needs long lane edges, which cost as much as a crossing
            lanes = sum(1 for ch in chains.values() if len(ch) > 2)
            best = (_score(net, pos, chains) + 10 * lanes, pos, chains)
            if method == "force" or best[0] >= 40:
                cands = [(_score(net, f, {}), f, {}) for f in (_rotate(_force(net, sd)) for sd in range(30))]
                top = min(cands, key=lambda c: c[0])
                best = top if method == "force" or top[0] <= best[0] / 2 else best
            _, pos, chains = best
    for n, (c, r) in ((hint or {}).get("positions", {})).items():
        if n in pos:
            pos[n] = (c * DX, r * DY) if rankdir == "LR" else (r * DY, c * DX)
    lo_x = min((p[0] for p in pos.values()), default=0)
    lo_y = min((p[1] for p in pos.values()), default=0)
    return {n: (x - lo_x, y - lo_y) for n, (x, y) in pos.items()}, chains


def _sugiyama(net: Net, rankdir: str):
    nodes = list(net.places) + list(net.transitions)
    layer, back = _layers(net)
    # layered edges with dummy nodes (every edge goes from a lower to a higher layer)
    chains: dict[tuple[str, str], list[str]] = {}
    lay = dict(layer)
    edges: list[tuple[str, str]] = []
    for s, t, _ in net.arcs:
        a, b = (t, s) if (s, t) in back else (s, t)
        if (a, b) in chains and (s, t) not in back:
            continue
        if layer[a] == layer[b]:  # cannot happen for a DAG with longest-path layers, defensive
            continue
        chain = [a]
        for k in range(layer[a] + 1, layer[b]):
            d = f"~{a}>{b}@{k}"
            lay[d] = k
            chain.append(d)
        chain.append(b)
        chains[(a, b)] = chain
        for u, w in zip(chain, chain[1:]):
            edges.append((u, w))
    maxl = max(lay.values(), default=0)
    cols: list[list[str]] = [[] for _ in range(maxl + 1)]
    for n in list(nodes) + [d for d in lay if d not in layer]:
        cols[lay[n]].append(n)
    up: dict[str, list[str]] = {n: [] for n in lay}
    down: dict[str, list[str]] = {n: [] for n in lay}
    for u, w in edges:
        down[u].append(w)
        up[w].append(u)

    def crossings() -> int:
        idx = {n: i for c in cols for i, n in enumerate(c)}
        total = 0
        for k in range(maxl):
            es = sorted((idx[u], idx[w]) for u, w in edges if lay[u] == k)
            for i in range(len(es)):
                for j in range(i + 1, len(es)):
                    if es[i][0] < es[j][0] and es[i][1] > es[j][1]:
                        total += 1
        return total

    best, best_c = [list(c) for c in cols], crossings()
    for sweep in range(12):
        rng = range(1, maxl + 1) if sweep % 2 == 0 else range(maxl - 1, -1, -1)
        nbr = up if sweep % 2 == 0 else down
        for k in rng:
            ref = {n: i for i, n in enumerate(cols[k - 1 if sweep % 2 == 0 else k + 1])}
            cur = {n: i for i, n in enumerate(cols[k])}

            def bary(n):
                ps = [ref[m] for m in nbr[n] if m in ref]
                return sum(ps) / len(ps) if ps else cur[n]
            cols[k].sort(key=bary)
        c = crossings()
        if c < best_c:
            best, best_c = [list(x) for x in cols], c
    cols = best
    # coordinates: each layer centred around zero, one row unit between neighbours
    pos: dict[str, tuple[float, float]] = {}
    for k, c in enumerate(cols):
        h = len(c)
        for i, n in enumerate(c):
            y = (i - (h - 1) / 2) * DY
            pos[n] = (k * DX, y) if rankdir == "LR" else (y, k * DX)
    # long edges run in one lane: all dummy nodes of an edge get the most outlying coordinate across the layers;
    # when a real node of one of these layers sits on that lane, the lane moves outside of it
    ax = 1 if rankdir == "LR" else 0
    for chain in chains.values():
        d = chain[1:-1]
        if not d:
            continue
        vals = [pos[c][ax] for c in d]
        if not (all(v >= 0 for v in vals) or all(v <= 0 for v in vals)):
            continue
        lane = max(vals, key=abs)
        side = 1 if lane >= 0 else -1
        layers = {lay[c] for c in d}
        real = [pos[n][ax] for n in nodes if lay[n] in layers]
        if any(abs(r - lane) < DY * 0.6 for r in real):
            lane = side * (max(r * side for r in real) + DY * 0.75)
        for c in d:
            q = list(pos[c])
            q[ax] = lane
            pos[c] = tuple(q)
    return pos, chains


# ------------------------------------------------------------------ drawing

def _clip(net: Net, n: str, c: tuple[float, float], toward: tuple[float, float]) -> tuple[float, float]:
    dx, dy = toward[0] - c[0], toward[1] - c[1]
    d = math.hypot(dx, dy) or 1.0
    ux, uy = dx / d, dy / d
    if net.is_place(n):
        r = R_PLACE
    else:
        h = S_TRANS / 2
        r = h / max(abs(ux), abs(uy), 1e-9)
    return c[0] + ux * r, c[1] + uy * r


def _smooth(points: list[tuple[float, float]], radius: float = 16.0) -> str:
    """Straight segments with rounded corners; points that lie (almost) on the line between their neighbours are dropped."""
    pts = [points[0]]
    for i in range(1, len(points) - 1):
        a, b, c = pts[-1], points[i], points[i + 1]
        cross = abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
        if cross / (math.dist(a, c) or 1.0) > 2.0:
            pts.append(b)
    pts.append(points[-1])
    d = f"M{pts[0][0]:.1f},{pts[0][1]:.1f}"
    for i in range(1, len(pts) - 1):
        a, v, b = pts[i - 1], pts[i], pts[i + 1]
        la, lb = math.dist(a, v) or 1.0, math.dist(v, b) or 1.0
        r = min(radius, la / 2, lb / 2)
        p1 = (v[0] + (a[0] - v[0]) / la * r, v[1] + (a[1] - v[1]) / la * r)
        p2 = (v[0] + (b[0] - v[0]) / lb * r, v[1] + (b[1] - v[1]) / lb * r)
        d += f" L{p1[0]:.1f},{p1[1]:.1f} Q{v[0]:.1f},{v[1]:.1f} {p2[0]:.1f},{p2[1]:.1f}"
    return d + f" L{pts[-1][0]:.1f},{pts[-1][1]:.1f}"


def _label_text(net: Net, n: str, mode: str, label_map: dict) -> str | None:
    if n in label_map:
        return label_map[n]
    if mode == "ids":
        return n
    if mode == "names":
        d = net.places.get(n) or net.transitions.get(n)
        return d["name"] or n
    return None


def render_body(net: Net, hint: dict | None = None, rankdir: str = "LR", labels: str = "none",
                label_map: dict | None = None, uid: str = "n", method: str = "auto") -> tuple[str, float, float]:
    """SVG elements of one net (without the outer <svg>), and its width and height including the margin."""
    label_map = label_map or {}
    raw_pos, chains = layout(net, hint, rankdir, method)
    pos = {n: p for n, p in raw_pos.items() if not n.startswith("~")}
    dummies = {n: p for n, p in raw_pos.items() if n.startswith("~")}
    allp = {**pos, **dummies}
    out: list[str] = []
    arc_set = {(a, b) for a, b, _ in net.arcs}
    for s, t, w in net.arcs:
        chain = None
        if (s, t) in chains:
            chain = chains[(s, t)]
        elif (t, s) in chains:
            chain = list(reversed(chains[(t, s)]))
        if chain and len(chain) > 2 and all(c in allp for c in chain):
            pts = [allp[c] for c in chain]
        else:
            pts = [pos[s], pos[t]]
            if (t, s) in arc_set:  # a read arc (one to, one back): both bend, to opposite sides
                mx, my = (pts[0][0] + pts[1][0]) / 2, (pts[0][1] + pts[1][1]) / 2
                nx, ny = -(pts[1][1] - pts[0][1]), pts[1][0] - pts[0][0]
                dd = math.hypot(nx, ny) or 1
                pts = [pts[0], (mx + nx / dd * 15, my + ny / dd * 15), pts[1]]
        a = _clip(net, s, pts[0], pts[1])
        b = _clip(net, t, pts[-1], pts[-2])
        # gap for the arrowhead is the marker itself; the path ends at the node border
        pts = [a] + pts[1:-1] + [b]
        out.append(f'<path d="{_smooth(pts)}" fill="none" stroke="#000" stroke-width="0.9" marker-end="url(#{uid}-arrow)"/>')
        if w != 1:
            i = len(pts) // 2
            lx, ly = ((pts[i - 1][0] + pts[i][0]) / 2, (pts[i - 1][1] + pts[i][1]) / 2) if len(pts) % 2 == 0 else pts[i]
            out.append(f'<text x="{lx:.1f}" y="{ly - 3:.1f}" font-size="8" text-anchor="middle" font-family="{FONT}" '
                       f'paint-order="stroke" stroke="#fff" stroke-width="2.5">{w}</text>')
    for n, (x, y) in pos.items():
        if net.is_place(n):
            out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{R_PLACE}" fill="#fff" stroke="#000" stroke-width="1"/>')
            m = net.places[n]["marking"]
            if 0 < m <= 4:
                offs = {1: [(0, 0)], 2: [(-3, 0), (3, 0)], 3: [(-3.2, 2), (3.2, 2), (0, -3)], 4: [(-3, -3), (3, -3), (-3, 3), (3, 3)]}[m]
                for ox, oy in offs:
                    out.append(f'<circle cx="{x + ox:.1f}" cy="{y + oy:.1f}" r="1.9" fill="#000"/>')
            elif m > 4:
                out.append(f'<text x="{x:.1f}" y="{y + 3:.1f}" font-size="9" text-anchor="middle" font-family="{FONT}">{m}</text>')
        else:
            h = S_TRANS / 2
            out.append(f'<rect x="{x - h:.1f}" y="{y - h:.1f}" width="{S_TRANS}" height="{S_TRANS}" fill="#fff" stroke="#000" stroke-width="1"/>')
        lab = _label_text(net, n, labels, label_map)
        if lab:
            ly = y + (R_PLACE if net.is_place(n) else S_TRANS / 2) + 9
            out.append(f'<text x="{x:.1f}" y="{ly:.1f}" font-size="8.5" text-anchor="middle" font-family="{FONT}" font-style="italic">{escape(lab)}</text>')
    xs = [p[0] for p in allp.values()] or [0]
    ys = [p[1] for p in allp.values()] or [0]
    w = max(xs) + 2 * MARGIN
    h = max(ys) + 2 * MARGIN + (10 if labels != "none" or label_map else 0)
    body = f'<g transform="translate({MARGIN},{MARGIN})">' + "".join(out) + "</g>"
    return body, w, h


def _defs(uid: str) -> str:
    return (f'<defs><marker id="{uid}-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
            f'orient="auto-start-reverse"><path d="M0,1 L10,5 L0,9 z" fill="#000"/></marker></defs>')


def render(net: Net, hint: dict | None = None, rankdir: str = "LR", labels: str = "none",
           label_map: dict | None = None, scale: float = 1.0, method: str = "auto") -> str:
    body, w, h = render_body(net, hint, rankdir, labels, label_map, method=method)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w * scale:.1f}" height="{h * scale:.1f}" '
            f'viewBox="0 0 {w:.1f} {h:.1f}">{_defs("n")}{body}</svg>\n')


def render_grid(items: list[tuple[str, Net]], cols: int = 3, rankdir: str = "LR", labels: str = "none",
                caption_size: float = 9.5, gap: float = 10.0, scale: float = 1.0,
                method: str = "auto") -> str:
    """Several nets as panels in a grid, each with a caption under it. Rows are as high as their tallest panel."""
    panels = []
    for i, (cap, net) in enumerate(items):
        body, w, h = render_body(net, None, rankdir, labels, None, uid=f"n{i}", method=method)
        panels.append((cap, body, w, h, i))
    rows = [panels[i:i + cols] for i in range(0, len(panels), cols)]
    colw = [max((r[c][2] for r in rows if c < len(r)), default=0) for c in range(cols)]
    out, y = [], 0.0
    for r in rows:
        rh = max(p[3] for p in r)
        x = 0.0
        for c, (cap, body, w, h, i) in enumerate(r):
            ox = x + (colw[c] - w) / 2
            oy = y + (rh - h) / 2
            out.append(f'<g transform="translate({ox:.1f},{oy:.1f})">{_defs(f"n{i}")}{body}</g>')
            out.append(f'<text x="{x + colw[c] / 2:.1f}" y="{oy + h + caption_size:.1f}" font-size="{caption_size}" '
                       f'text-anchor="middle" font-family="{FONT}">{escape(cap)}</text>')
            x += colw[c] + gap
        y += rh + caption_size + gap + 4
    W, H = sum(colw) + gap * (cols - 1), y - gap
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W * scale:.1f}" height="{H * scale:.1f}" '
            f'viewBox="0 0 {W:.1f} {H:.1f}">{"".join(out)}</svg>\n')


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("pnml")
    ap.add_argument("-o", "--out", help="output SVG (default: stdout)")
    ap.add_argument("--rankdir", choices=["LR", "TB"], default="LR")
    ap.add_argument("--labels", choices=["none", "ids", "names"], default="none")
    ap.add_argument("--label-map", help="JSON file {node id: label}")
    ap.add_argument("--hint", help="JSON file {\"positions\": {node id: [col, row]}}")
    ap.add_argument("--layout", choices=["auto", "layered", "force", "ring"], default="auto",
                    help="auto: a ring for buffer rings, else layered unless a spring layout is clearly cleaner")
    ap.add_argument("--scale", type=float, default=1.0)
    a = ap.parse_args(argv)
    net = parse_pnml(open(a.pnml, encoding="utf-8").read())
    svg = render(net, json.load(open(a.hint)) if a.hint else None, a.rankdir, a.labels,
                 json.load(open(a.label_map)) if a.label_map else None, a.scale, a.layout)
    if a.out:
        open(a.out, "w", encoding="utf-8").write(svg)
    else:
        sys.stdout.write(svg)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
