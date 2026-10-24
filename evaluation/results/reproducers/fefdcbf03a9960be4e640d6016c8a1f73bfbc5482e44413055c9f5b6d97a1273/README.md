# Reproducer fefdcbf03a9960be4e640d6016c8a1f73bfbc5482e44413055c9f5b6d97a1273

- outcome: DISAGREE
- pattern: pv-modes-disagree
- campaign: A
- instance_id: bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QR
- target: f8bebcb7f2980b2f, marking [0,1,0,0,0,0,0,1]
- truth: TRUE (source lola+tina)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0.pnml -p reach -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/dada1c5546791254.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (98c0a669a741ec551a4d2eac59c8c81e232ad0d9349ecc037c0e742b44a7ebe2)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0.pnml","property":"reach","mode":"dispatch","verdict":"false","class":"General","places":8,"transitions":13,"arcs":48,"parse_ms":0.237,"analyze_ms":11.570,"timeout_ms":null}

```

## stderr

(none)
