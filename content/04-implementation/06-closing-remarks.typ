== Closing Remarks

The dispatch procedures implemented in `petrivet` form the analytical backbone of the library,
and are constantly being improved, extended, and re-evaluated.

There are several known procedures which are not yet implemented but are on the author's roadmap.

[Rank theorem: positive P- and T-invariants exist, $"rank"(C) = c - 1$ with $c$ the number of clusters, and every proper siphon is marked at $M_0$],
[$⇔$ ("live and bounded")],
[P; $O(|P| * |T| * |F|)$ for the structural part],
[@esparza_petri_2026[Thm. 5.3.16]; @best_petri_2024[Lem. 6.17, Thm. 6.18]; @esparza_analysis_1991[Cor. 2.8]; @kemper_efficient_1992],

The Commoner/Hack criterion also needs re-evaluation;
its high theoretical complexity often translates to poor performance in practice,
as the evaluation will show in @eval-dispatch.

`petrivet` currently does not support any form of caching of state spaces.
This particularly disastrous for Petri nets with less restricted structure and large state spaces,
where the coverability graph fallback procedure is activated for the majority of queries
and has to be restarted from scratch each time.
Furthermore, there is no way to construct the set of reachable markings without
also constructing the graph structure, which is sometimes unnecessary and wasteful for certain queries.
Future work will address this by providing a method to construct the set of reachable markings directly.

Also, future work will investigate partial-order reduction techniques to skip the exploration of parts
of the state space without losing information relevant to the property being checked.