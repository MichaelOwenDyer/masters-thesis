# Reproducer 82f7327feb14519377dc3940a067ff1f4285b28bb5e2113252d18f13c687f420

- outcome: NO_TRUTH
- pattern: pv-modes-disagree
- campaign: A
- instance_id: 8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889.pnml
- instance copy: instance.pnml
- solver: pv-general
- query: QR
- target: 1eeab3daf51afc68, marking [0,0,1,0,3,1]
- truth: none (source -)
- answer: TRUE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889.pnml -p reach -m general --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/3b4ca27a7191749f.json
```

## Other solvers on this cell

- pv-dispatch: FALSE (NO_TRUTH)
- pv-poly: no answer (UNKNOWN)

## Raw output (ccb4bad6f9d2b46843112e7b65b8f945dcdf08e0b55305e0ccfdc935961ca659)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889.pnml","property":"reach","mode":"general","verdict":"true","class":"General","places":6,"transitions":8,"arcs":26,"parse_ms":0.201,"analyze_ms":0.044,"timeout_ms":null}

```

## stderr

(none)
