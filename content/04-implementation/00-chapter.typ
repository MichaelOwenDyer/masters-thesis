= Implementation <implementation>

This section discusses the implementation of the _petrivet_ #footnote[https://github.com/MichaelOwenDyer/petrivet] Rust
crate, including arch

#include "01-builder.typ"
#include "02-simulation.typ"
#include "03-state_space.typ"

=== Fundamental Property Checking

The fundamental properties of interest for Petri nets are:
- boundedness,
- deadlock-freedom,
- liveness,
- coverability, and
- reachability.

petrivet provides methods to check each of these properties.
At this point, we would be missing a major opportunity if we did not consider the structural subclass of the net when checking these properties,
as the computational complexity varies significantly depending on the subclass.
petrivet dispatches to the most efficient known procedure available for the net on a best-effort basis.
When the net is a subclass for which no efficient procedure is known, petrivet will fall back to the general procedure for arbitrary nets.
This may not be ideal for all users, especially when analyzing nets of unknown magnitude, as the general procedures are often prohibitively expensive.
Therefore petrivet also provides incomplete methods for checking these properties using only efficient procedures, insofar as they are known and implemented,
and will return `None` if no efficient procedure is known for the net.
This gives the user the choice to check for these properties quickly if possible, or to check for them exhaustively
if they are willing to pay the computational cost in the general case.

Each of these properties is implemented as a ladder of increasingly expensive procedures,
such that the cheapest procedures are attempted first, and the more expensive procedures are only attempted if the
cheaper ones fail to provide an answer. Some procedures are complete, meaning they will always return a definitive answer,
while others are incomplete, particularly for unbounded nets when the final fallback procedure is to construct the reachability graph.
More work is needed to provide complete procedures for unbounded nets, and this is a major area of future work for petrivet.

=== Use Case Dimensions

The API of petrivet aims to cover as many use cases as possible.
I identified three dimensions along which the user's needs may vary, and designed the API to accommodate these variations.

+ Property Scope
  Boundedness and liveness have overloaded definitions in the theory: they are properties both of individual nodes
  (places and transitions, respectively) and of the entire Petri net. Depending on which definition the user is interested in,
  they may want to check the property for a specific node or for the entire system.
+ Desired result granularity
  The user may want simply a yes/no answer to whether the property holds, or they may want to know more detailed information,
  e.g. the exact bound for boundedness, or the set of deadlocked markings for deadlock-freedom.
+ Efficiency restrictions
  The computational complexity and thus the expected runtime of checking these properties varies significantly
  depending on the characteristics of the Petri net.
  A general purpose method for checking a property may return an answer in a reasonable amount of time for some nets,
  but take prohibitively long (effectively forever) for others.
  This is inconvenient for the user, who may not know what kind of net they are analyzing and what kind of computational cost to expect.
  Therefore, petrivet offers two versions of each method: a general-purpose version which attempts to compute the property
  using any known procedure, and an efficient-only version which returns an answer only if it can be computed with
  a polynomial-time procedure. The return type of the efficient-only version is the same as the general-purpose version,
  wrapped in `std::option::Option`.
  A return value of `Option::None` indicates that no efficient procedure is known and no attempt was made to compute the property,
  returning early instead of potentially wasting the user's time on a long-running computation.

An overview of the procedures implemented in petrivet for checking these properties is provided in the following sections.
For each property, a table of theorems is provided, summarizing known results from the literature and their implementation status in petrivet.
Additionally, the three dimensions of property scope, desired result granularity, and efficiency restrictions are discussed for each property.
Finally, the petrivet API for checking the property is presented, reflecting a best-effort attempt to accommodate the various combinations of these dimensions,
and with reference to the theorems table for the underlying procedures used in each case.

#include "04-boundedness.typ"
#include "05-liveness.typ"
#include "06-deadlock-freedom.typ"
#include "07-reachability-coverability.typ"
#include "08-chc.typ"
