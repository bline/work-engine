# P1.34 coordinator pre-registration (written before the Module A response exists)

These are the weaknesses of revision 5 (`1.4-s1`) as the coordinator (the author of revisions 1 to 5) sees them. They
are recorded so that the independent result can be compared without hindsight.

1. **Shared author.** One author wrote the case builder (`rev5_lib.finalize`), the validator and the contract. Their
   agreement shows consistency, not independent correctness.
2. **Raw re-derivation is partial.** The validator re-derives status, the model rule, `R` and the OpenAI range from raw
   rows. It does not re-derive Anthropic valuation, the `Undercharge` arithmetic or multi-identity valuation.
3. **A-02 reachability is not established.** No producer path writing usage with a non-200 status is known. The
   error-status-with-usage case is synthetic.
4. **Gate authenticity is outside S1.** G-8 checks a binding digest that a self-asserting writer can compute. Signature
   verification is deferred to stage E of the future vertical.
   - **Prediction:** a reviewer may judge `P133-FG-VC14-authenticated-bound` self-asserted (the challenge raised in
     §5 of the P1.34 instruction).
   - **Confidence that the contract states the authority requirement clearly enough:** moderate at best.
5. **The `billing` fact is narrow.** `billing = established` is set by a valid usage settlement in the provisional
   measure. Whether that name can be read as "the provider invoice is final" is the exposure-versus-billing challenge.
   - **Prediction:** at least a P2 on naming or consumer guarantees is plausible.
6. **Correlated reservations.** X-0 takes the largest distinct reservation for one `rid`. Distinct attempts are
   separated by key. Whether correlation by `rid` alone could merge distinct attempts under a reused `rid` (R04) is
   arguable.
7. **Proxy valuation without cache counts.** The deployed proxy values missing cache counts at the dearest rate. The
   contract enumerates vertices instead. The two measures differ by design; the contract treats the proxy charge as a
   proxy amount.
8. **Coverage.** The packet samples 56 of 173 cases.
9. **Architecture guard.** The subsystem's guard (`ARCHITECTURE-GUARD.md`) is TRIGGERED before this review. Whatever
   the verdict, ordinary repair needs an Astra xhigh architectural review first.

**Coordinator expectation (not shared with the reviewer):** REJECT or ACCEPT WITH RESERVATIONS are both plausible.
**Confidence in an ACCEPT with no P1:** low. The prior rejection rate of this subsystem is the basis, not a specific
known defect.
