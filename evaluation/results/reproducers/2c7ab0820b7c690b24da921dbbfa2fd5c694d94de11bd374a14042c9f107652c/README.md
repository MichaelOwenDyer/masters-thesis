# Reproducer 2c7ab0820b7c690b24da921dbbfa2fd5c694d94de11bd374a14042c9f107652c

- outcome: DISAGREE
- pattern: pv-modes-disagree
- campaign: A
- instance_id: bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QR
- target: 09893c7576910921, marking [0,1,0,1,0,1,0,0]
- truth: TRUE (source lola+tina)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0.pnml -p reach -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/c84f0ebc6d6cc184.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (95471d94c217569999f47084a89d384db705365db711fe9f4f5754c5fb21bdca)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/bb42e878daa16f4387c801a5026a1d2e2744c2e89fc4e232fe13d477bb9e66e0.pnml","property":"reach","mode":"dispatch","verdict":"false","class":"General","places":8,"transitions":13,"arcs":48,"parse_ms":0.265,"analyze_ms":11.681,"timeout_ms":null}

```

## stderr

(none)
