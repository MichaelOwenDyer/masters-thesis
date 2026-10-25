// The numbers and tables of the evaluation chapter. Everything is read from the files that the analysis wrote
// (evaluation/export/): the text and the data cannot drift apart. Keys are the keys of numbers.json (see
// evaluation/analysis), tables are the CSV files of the export.

// `typst compile --input export-dir=/target/export-dev/ ...` reads another export (for development)
#let export-dir = sys.inputs.at("export-dir", default: "/evaluation/export/")
#let nums = json(export-dir + "numbers.json")

// the value of a key of numbers.json; fails loudly (at compile time) if the key does not exist
#let v(key) = {
  assert(key in nums, message: "numbers.json has no key " + key)
  nums.at(key)
}

// an integer with thousands separators: 13246 -> 13,246
#let group(x) = {
  let s = str(calc.abs(calc.round(x)))
  let n = s.len()
  let out = ""
  for (i, c) in s.clusters().enumerate() {
    if i > 0 and calc.rem(n - i, 3) == 0 { out += "," }
    out += c
  }
  if x < 0 { "−" + out } else { out }
}

// a number with d decimals (thousands separators in the integer part)
#let dec(x, d: 1) = {
  if d == 0 { return group(x) }
  let r = calc.round(float(x), digits: d)
  let whole = calc.trunc(calc.abs(r))
  let frac = str(calc.round((calc.abs(r) - whole) * calc.pow(10, d)))
  while frac.len() < d { frac = "0" + frac }
  (if r < 0 { "−" } else { "" }) + group(whole) + "." + frac
}

#let pct(x, d: 1) = dec(x * 100, d: d) + "%"

// a number of the table cell: empty stays a dash, whole numbers are grouped, others get d decimals
#let cell(x, d: 1) = {
  if x == "" or x == none { return [–] }
  let f = float(x)
  if f == calc.round(f) and calc.abs(f) < 1e15 { return [#group(f)] }
  [#dec(f, d: d)]
}

// the rows of a CSV of the export as dictionaries
#let rows(file) = csv(export-dir + file, row-type: dictionary)

// the first row of a CSV that matches all (column: value) pairs; fails if there is none
#let find(file, ..conditions) = {
  let c = conditions.named()
  let hits = rows(file).filter(r => c.pairs().all(((k, val)) => r.at(k) == val))
  assert(hits.len() > 0, message: "no row in " + file + " for " + repr(c))
  hits.first()
}

// names of the tools and the collections, in plain language
#let solver-name = (
  "pv-dispatch": "automatic choice",
  "pv-general": "plain exploration",
  "pv-poly": "fast methods only",
  "lola": "LoLA",
  "tina": "TINA",
  "its-tools": "ITS-Tools",
)

#let source-name = (
  "gen": "Generated nets (two tools)",
  "gen-lola-only": "Generated nets (LoLA alone)",
  "fam": "Scaling families",
  "mcc-small": "Competition models, small",
  "mcc-large": "Competition models, large",
  "calibration": "Hand-checked nets",
)

#let class-name = (
  "circuit": "circuit",
  "sm": "state machine",
  "mg": "marked graph",
  "efc": "extended free-choice",
  "ac": "asymmetric choice",
  "general": "general",
)

#let question-name = (
  "Q0": "size of the state space",
  "Q1": "deadlock-freedom",
  "Q2": "boundedness",
  "Q3": "liveness",
  "Q4": "quasi-liveness",
  "Q5": "1-safety",
  "Q6": "stable place",
  "QR": "reachability",
  "QC": "coverability",
)

// the look of the tables of the chapter
#let eval-table(columns, header, ..body, size: 8pt, align: auto) = text(size: size, table(
  columns: columns,
  align: if align == auto { (left,) + (right,) * (columns.len() - 1) } else { align },
  inset: 3.5pt,
  stroke: 0.4pt + luma(170),
  table.header(..header.map(h => [*#h*])),
  ..body,
))

#let family-name = (
  "circuit": "circuit (plain ring)",
  "sm_hub_and_spokes": "hub and spokes",
  "sm_two_cycles": "two cycles, one-way switch",
  "sm_two_cycles_n_tokens": "two cycles, n tokens",
  "sm_choice_ring": "ring of choices",
  "sm_ring_with_sink": "ring with a token sink",
  "mg_buffer_ring": "ring of buffers",
  "mg_buffer_ring_unmarked_circuit": "ring of buffers, token-free circuit",
  "mg_forkjoin_cycle": "fork and join, cyclic",
  "mg_forkjoin_unbounded": "fork and join, unbounded",
  "efc_forkjoin": "fork and join with choice",
  "efc_forkjoin_leaky": "fork and join with choice, leaky",
  "ac_cycle": "cycle with choices",
  "ac_cycle_no_low_token": "cycle with choices, no start token",
  "general_cycle": "cycle (general net)",
)

// a p-value for a table: tiny values as "< 0.001"
#let p-value(x) = {
  let f = float(x)
  if f < 0.001 [< 0.001] else [#dec(f, d: 3)]
}
