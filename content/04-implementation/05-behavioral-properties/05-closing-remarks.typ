The dispatch procedures implemented in `petrivet` form the analytical backbone of the library,
and are constantly being improved, extended, and re-evaluated.

There are several known procedures which are not yet implemented but are on the author's roadmap.

[Rank theorem: positive P- and T-invariants exist, $"rank"(C) = c - 1$ with $c$ the number of clusters, and every proper siphon is marked at $M_0$],
[$⇔$ ("live and bounded")],
[P; $O(|P| * |T| * |F|)$ for the structural part],
[@esparza_petri_2026[Thm. 5.3.16]; @best_petri_2024[Lem. 6.17, Thm. 6.18]; @esparza_analysis_1991[Cor. 2.8]; @kemper_efficient_1992],

The Commoner/Hack Criterion also needs re-evaluation;
its high theoretical complexity often translates to poor performance in practice,
as the evaluation will show in @eval-dispatch.