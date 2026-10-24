# Captured tool outputs (verbatim)

Captured on the NixOS evaluation box (2026-10-23) with LoLA 2.0 and TINA 4.0.0 on two tiny nets and one MCC model.
`cyc`: places p0 (1 token), p1; transitions t0: p0 -> p1, t1: p1 -> p0 (live, deadlock-free, 2 states, 2 edges).
`dead`: the same ring with no token (deadlock in the initial marking, 1 state, 0 edges, t0 and t1 dead).
`anderson`: MCC model Anderson-PT-04 (oracle 2026: 29641 states, 97516 edges, live, quasi-live, 1-safe, no deadlock, no stable place).
All commands exited 0 unless the file name says otherwise. These files are test inputs for the parsers of the LoLA and TINA adapters.

## ITS-Tools (`its_*.txt`)

Lines copied from the output of `its-tools-flat.sh -pnfolder DIR -examination NAME -its -timeout N` (product of 2026-09-11,
SHA-256 `faef344a…bae5f`) in the spike of 2026-10-24: the verdict line and the lines next to it, from the calibration net
`cal-01-circuit-live` and the MCC model `SmallOperatingSystem-PT-MT0016DC0008`, put together into small files (the files
are not complete outputs).
`its_ReachabilityDeadlock_true.txt` is the same line with the verdict flipped to cover both directions of the inversion
(the contest asks whether a deadlock is reachable).
