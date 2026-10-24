# Reproducer 3d95942fcef39b27e254784260846010b6f06857562b9a48b92dc0c68faed86e

- outcome: DISAGREE
- pattern: petrivet-vs-single-tool
- campaign: A
- instance_id: e171d6649ba2ce3b7348d7c27cd461dfc238e94bda853992f7b19788344c69a1
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/e171d6649ba2ce3b7348d7c27cd461dfc238e94bda853992f7b19788344c69a1.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QR
- target: 0705326775245d81, marking [1,0,0,0,3]
- truth: TRUE (source lola-only)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/e171d6649ba2ce3b7348d7c27cd461dfc238e94bda853992f7b19788344c69a1.pnml -p reach -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/7ac72e59dc27f85d.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (f714ebe076c03864cf0cde941a4d8a6f15132776e27809342e7aa97e07a3adb7)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/e171d6649ba2ce3b7348d7c27cd461dfc238e94bda853992f7b19788344c69a1.pnml","property":"reach","mode":"dispatch","verdict":"false","class":"General","places":5,"transitions":9,"arcs":41,"parse_ms":0.209,"analyze_ms":11.438,"timeout_ms":null}

```

## stderr

(none)
