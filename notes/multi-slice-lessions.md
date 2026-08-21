We learned quite a lot, and I think the biggest lesson is that the long-lived supervisor experiment worked well enough to change our default assumptions.

The clearest findings are these:

* **A persistent supervisor context can survive many slices without obvious quality collapse.** Fourteen completed slices, with the fifteenth in progress, is already enough to show that “fresh supervisor every slice” is not obviously necessary.
* **The scary token totals are mostly cache reuse, not fresh context growth.** Builder input rose sharply in headline numbers, but by slices 13–14 roughly 95–97% of that input was cached. The uncached component stayed around ~90K–135K rather than exploding.
* **Persistent context appears to be buying us accumulated understanding.** Later slices did not require reconstructing the entire campaign state from scratch. That supports the idea that context has value as retained interpretation, not merely as storage.
* **Review cost is highly uneven and should remain judgment-driven.** Some slices needed multiple expensive independent reviews; others needed none. A uniform review procedure would have wasted a lot of tokens.
* **Reviewer persistence is now even more compelling.** Across the run we saw 33 independent-review invocations and a large amount of repeated cache/read/output work. Keeping a fresh reviewer for initial independence but reusing it through remediation still looks like one of the biggest remaining efficiency opportunities.
* **Capability selection and fallback provenance need to remain separate.** The run demonstrated a real evidence fallback when indexed evidence could not establish external host-artifact facts. That fallback deserved provenance, but it did not inherently require a provider change once both evidence capabilities could coexist.
* **The workflow benefits from model judgment precisely where procedures would have hurt it.** The skill-refresh episode is a perfect small example: instead of restarting and risking context loss, the supervisor realized the builder could simply reread the changed skills.
* **Our new design language fits the observed behavior better than the old procedural framing.** Invariant structure removed invalid states; the machinery gave the agents useful capabilities; the model chose routes based on current evidence. The useful flexibility happened inside those bounds.

There is also one important measurement gap:

**we still do not have equivalent per-slice supervisor token telemetry.**

We know the builder children share the same parent supervisor thread, but we cannot yet plot something like:

```text
slice 1 supervisor context
slice 2 supervisor context
...
slice 15 supervisor context
```

That is probably the next metric worth adding. It would let us distinguish:

* beneficial accumulated supervisor knowledge,
* harmless cached replay,
* genuinely growing live context burden,
* and the point, if any, where persistence begins hurting decision quality or latency.

So I think the strongest overall conclusion is:

> **Context should not be discarded merely because a unit of work ended. Its lifetime should follow the lifetime of useful reasoning.**

That applies differently to each role:

* supervisor context can span many related slices;
* builder context should persist while implementation knowledge is still relevant;
* reviewer context should begin fresh for independence, then persist through remediation;
* reconnaissance context can usually die after returning compressed evidence.

And perhaps the most important philosophical lesson from this run is:

> **Persistence itself is not the enemy. Uncontrolled context with no continuing semantic value is.**

That is a much more useful design target than simply trying to minimize context size.

