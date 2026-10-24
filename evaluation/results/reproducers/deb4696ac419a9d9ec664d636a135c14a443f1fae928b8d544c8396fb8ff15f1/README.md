# Reproducer deb4696ac419a9d9ec664d636a135c14a443f1fae928b8d544c8396fb8ff15f1

- outcome: DISAGREE
- pattern: pv-modes-disagree
- campaign: A
- instance_id: f2b3390663e52d86b310d1972848363cf787cb7b53c1160ce10b94546938689f
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/f2b3390663e52d86b310d1972848363cf787cb7b53c1160ce10b94546938689f.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QC
- target: 079123d6e9177048, marking [1,0,0,0,1,0,0,0,0,1]
- truth: TRUE (source lola+tina)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/f2b3390663e52d86b310d1972848363cf787cb7b53c1160ce10b94546938689f.pnml -p cover -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/734f3cf17d3d4964.json
```

## Other solvers on this cell

- pv-general: TRUE (AGREE)
- pv-poly: no answer (UNKNOWN)

## Raw output (0498ccafb4d89e8897b03cf3bd355becf48ae74641a718512ddac02c15dc0403)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/f2b3390663e52d86b310d1972848363cf787cb7b53c1160ce10b94546938689f.pnml","property":"cover","mode":"dispatch","verdict":"false","class":"General","places":10,"transitions":20,"arcs":86,"parse_ms":0.315,"analyze_ms":14.084,"timeout_ms":null}

```

## stderr

(none)
