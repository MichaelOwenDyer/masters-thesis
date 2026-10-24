# Reproducer 46df52e58aa55232e99b9ed9fa491bc4670bca2087a5ddf18a015d5dba9ce044

- outcome: NO_TRUTH
- pattern: pv-modes-disagree
- campaign: A
- instance_id: 8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889
- instance path: /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889.pnml
- instance copy: instance.pnml
- solver: pv-dispatch
- query: QR
- target: 1eeab3daf51afc68, marking [0,0,1,0,3,1]
- truth: none (source -)
- answer: FALSE

## Command

```
petrivet analyze /home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889.pnml -p reach -m dispatch --json --target /home/michael/RustroverProjects/masters-thesis/evaluation/results/targets/3b4ca27a7191749f.json
```

## Other solvers on this cell

- pv-general: TRUE (NO_TRUTH)
- pv-poly: no answer (UNKNOWN)

## Raw output (3564a862ff6a19f0df4f8aed6b05e70c0a7febb6fbd45d70c6ea472dc7670cf4)

```
{"tool":"petrivet","version":"0.1.0","file":"/home/michael/RustroverProjects/masters-thesis/evaluation/corpus/gen/8f96f2555aa16130817ed0f89e664728d9546a397c4d2272b877914098c0e889.pnml","property":"reach","mode":"dispatch","verdict":"false","class":"General","places":6,"transitions":8,"arcs":26,"parse_ms":0.199,"analyze_ms":10.917,"timeout_ms":null}

```

## stderr

(none)
