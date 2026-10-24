# Reproducer d652d756ab43c59de4a041ea9f0e814685b3d31fc68adf1664fcfafd8c76ac93

- outcome: DISAGREE
- pattern: petrivet-vs-single-tool
- campaign: A
- instance_id: aa5fc9d63af8f11c154e26c112dd92e0f649794590fdd3bdac90464c197cb36a
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/aa5fc9d63af8f11c154e26c112dd92e0f649794590fdd3bdac90464c197cb36a.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QR
- target: 2926daad5f927158, marking [0,0,1,1,0,2,0,0,0]
- truth: TRUE (source lola-only)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/aa5fc9d63af8f11c154e26c112dd92e0f649794590fdd3bdac90464c197cb36a.pnml -p reach -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/bb38193f55de0439.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (5461f03e4f19e5cb7c73c02917d076f722c9c2e40e090e5b8a24715a30dadd5d)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/aa5fc9d63af8f11c154e26c112dd92e0f649794590fdd3bdac90464c197cb36a.pnml","property":"reach","mode":"dispatch","verdict":"false","class":"General","places":9,"transitions":14,"arcs":58,"parse_ms":0.278,"analyze_ms":13.045,"timeout_ms":null}

```

## stderr

(none)
