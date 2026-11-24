# Held-start contract

## Evidence owner and usability

The Host owns the child, same-session framed handshake, and exact stop.

## Terminal cleanup

The explicitly accepted cancel for the owned, still-held session concludes only after bounded receiver exit collection and exact container removal observed against the owned identity.

**Owner decision (receipt-semantics owner, recorded in this contract):** Clean cancel additionally requires that the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal. Removal by any other actor does not satisfy clean cancel, even if the container is afterwards absent. An independent post-exit check that the original container is absent is also required.
