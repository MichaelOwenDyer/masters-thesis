# Reproducer ea6989b8e183495a84f1d1c7f3c3230adebbec0066c62cd78f7813de6a3173ee

- outcome: DISAGREE
- pattern: pv-modes-disagree
- campaign: A
- instance_id: b3758a0ea28b8980633099abb8305a8165f20e16c4cdab295e543e596c03a13b
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/b3758a0ea28b8980633099abb8305a8165f20e16c4cdab295e543e596c03a13b.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QC
- target: 291b585e9bc4c13a, marking [1,2,1,0,0,0,0,0,0,1]
- truth: TRUE (source lola+tina)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/b3758a0ea28b8980633099abb8305a8165f20e16c4cdab295e543e596c03a13b.pnml -p cover -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/5486ef66a6f93833.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (85f11f130cf10927fb1992f0e18863d995e5db481c4a20ba2cfdd3a5260f84d1)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/b3758a0ea28b8980633099abb8305a8165f20e16c4cdab295e543e596c03a13b.pnml","property":"cover","mode":"dispatch","verdict":"false","class":"General","places":10,"transitions":8,"arcs":40,"parse_ms":0.219,"analyze_ms":16.558,"timeout_ms":null}

```

## stderr

(none)
