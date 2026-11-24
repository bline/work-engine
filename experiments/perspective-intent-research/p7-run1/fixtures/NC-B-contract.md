# Held-start contract with explicit removal attribution (synthetic fixture NC-B)

Status: constructed negative control for P7 run 1. Not a real Work Engine contract. Derived from the shape of a held-start contract, with one owner decision added.

## Evidence owner and usability

The Host owns the child, same-session framed handshake, and exact stop.

## Terminal cleanup

The explicitly accepted cancel for the owned, still-held session concludes only after bounded receiver exit collection and exact container removal observed against the owned identity.

**Owner decision (receipt-semantics owner, recorded in this contract):** Clean cancel additionally requires that the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal. Removal by any other actor does not satisfy clean cancel, even if the container is afterwards absent. An independent post-exit check that the original container is absent is also required.
