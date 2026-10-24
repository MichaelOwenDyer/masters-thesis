# Reproducer 28b248a5066e84dd808a35463b2d2f39173c68f7e6e5eb8136bda3827b386f2d

- outcome: DISAGREE
- pattern: petrivet-vs-single-tool
- campaign: A
- instance_id: 8c5b2d43291fed33b55b08773cd9c6220ff67429128e1ae6e82cffa264bd2150
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8c5b2d43291fed33b55b08773cd9c6220ff67429128e1ae6e82cffa264bd2150.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QC
- target: c92406f183f41340, marking [2,0,0,0,0,0,0,0,0,0]
- truth: TRUE (source lola-only)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8c5b2d43291fed33b55b08773cd9c6220ff67429128e1ae6e82cffa264bd2150.pnml -p cover -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/1e680e528a5aad8a.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (98997feb93b86c4a3f335cfc6ef5222450e4bd3f227d7e1df06e4f49fdad9058)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8c5b2d43291fed33b55b08773cd9c6220ff67429128e1ae6e82cffa264bd2150.pnml","property":"cover","mode":"dispatch","verdict":"false","class":"General","places":10,"transitions":20,"arcs":77,"parse_ms":0.299,"analyze_ms":12.688,"timeout_ms":null}

```

## stderr

(none)
