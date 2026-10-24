# Reproducer 6a1c0527c65d4ae24be5da8e5c5d1e9975c0d8ac0ac6be19b84c792121f3d229

- outcome: DISAGREE
- pattern: pv-modes-disagree
- campaign: A
- instance_id: e30bbc543d1df793db108a70e74a9c48e85c09b8de533a3c31e7f030552e9e0e
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/e30bbc543d1df793db108a70e74a9c48e85c09b8de533a3c31e7f030552e9e0e.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QC
- target: ec6c862ffc4a3fc7, marking [0,1,0,2,0,0,0,0,0,0,0,0]
- truth: TRUE (source lola+tina)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/e30bbc543d1df793db108a70e74a9c48e85c09b8de533a3c31e7f030552e9e0e.pnml -p cover -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/1bc6687cb8a2ef43.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (132c76839536b978dcba109863515cccbc315223adfba4f588e74889e18ab46b)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/e30bbc543d1df793db108a70e74a9c48e85c09b8de533a3c31e7f030552e9e0e.pnml","property":"cover","mode":"dispatch","verdict":"false","class":"FreeChoice","places":12,"transitions":16,"arcs":83,"parse_ms":0.313,"analyze_ms":13.175,"timeout_ms":null}

```

## stderr

(none)
