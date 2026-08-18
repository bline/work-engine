Yeah — I like this architecture a lot. The important part is that you’ve separated **durable reasoning from disposable evidence gathering**. That is a much better model for agentic coding than “give one giant agent the repository and keep feeding it until context death.”

The supervisor → persistent slice builder → disposable recon/reviewer topology is sound. I would keep it.

Where I think you can make a surprisingly large improvement is that your **recon agent is currently doing too much reasoning and reporting**. It’s halfway between a retrieval agent and another architect.

Your own metrics make that visible. The Visual Evidence slice used four Claude calls, 13 files / 22 ranges / 3,437 retrieved lines, but Claude emitted **83,530 output tokens**. That’s an average of roughly **21k output tokens per call**. The 7 million cached-read tokens look scary, but caching changes the economics; the 83k generated tokens are the part that really jumps out at me. And you paid about $5.37 in the recorded provider cost for that slice.

I think you can probably cut external-model generation by **50–80% without weakening the workflow**.

The biggest changes I would make are:

* **Turn recon back into reconnaissance.** Your targeted-recon schema currently asks for `observed_state`, `semantic_effect_path`, `selected_boundary`, `call_path`, `module_and_test_wiring`, `invariants`, `acceptance_tests`, `vertical_semantic_test`, `test_commands`, vocabulary, deferred scope, open decisions, missing-context risk, etc. That is basically a miniature architecture report. I would have recon return only the evidence necessary for Codex to reason: placement verdict, a few facts that prove/falsify the certificate, exact ranges, wiring/commands that Codex cannot cheaply discover, and blockers. Let the persistent builder derive invariants, acceptance tests, changed-file boundaries, and the final plan.
* **Remove the LLM from ordinary test execution.** This is probably the single easiest win. Claude does not need to run `git diff --check`, freshness checks, focused tests, and the 582-test suite. A deterministic gate script can do all of that and return a tiny structured result. Invoke a model only if something fails and needs diagnosis, or after everything passes for adversarial semantic review. You retain all of the safety with dramatically less model activity.
* **Don’t rerun the entire expensive gate before every review iteration.** I’d do vertical proof + focused checks → adversarial review → fixes → affected checks → **one final full suite/freshness gate after the last fix**. You absolutely want the final full suite after review fixes, but Claude doesn’t need a clean full-suite run before it can notice “you persisted the wrong thing.”
* **Split audit receipts from context handoffs.** Your durable receipt is excellent for metrics, but it is too rich to be the thing passed to the next builder. In particular, `engine_config`, provider statistics, validation bookkeeping, and model metadata have essentially zero architectural value to the next slice. I’d have an `audit_receipt` and a tiny `handoff_receipt`: what became true, durable architectural decisions made, affected boundaries, unresolved concerns, deferred work. Maybe 300–800 tokens. The supervisor can retain the full JSONL record without feeding it back into model context.
* **Have one canonical contract instead of restating it at every layer.** There’s considerable semantic duplication among `slice-supervisor/SKILL.md`, `slice-builder/SKILL.md`, `builder-receipt.md`, `receipt-schema.md`, and `claude-recon-implementation/SKILL.md`. That duplication is good defensive engineering initially, but the builder is being told variations of the same placement-certificate/vertical-proof/rejected-alternative requirements several times. A shared compact contract would save prompt tokens and, perhaps more importantly, reduce eventual contract drift.

That last one is already showing a tiny bit of drift: the receipt documentation says new placement-first runs should emit schema version 3, while the two stored metrics records are schema version 2. There are also old/current path differences in the recorded configs. None of that is alarming while you’re building this, but it demonstrates why I’d want fewer independently repeated definitions.

### The recon model does not need to be Claude

I actually think **Claude is most valuable at the opposite end of this pipeline**.

For your workflow, I’d assign model capability something like:

**Placement scout:** cheap/fast tool-using model, low reasoning. Its job is essentially “find the plausible homes and the evidence discriminating them.”

**Targeted recon:** cheap/fast model again. This is mostly grep/read/call-path tracing. Escalate only when evidence is genuinely ambiguous.

**Codex builder:** your current persistent GPT-5.6 Sol low-effort arrangement makes sense. It is where architectural context has durable value.

**Test runner:** no model.

**Failure triage:** cheap model first if the failure isn't obvious; Codex builder handles the actual fix.

**Adversarial review:** this is where I would spend your Claude allowance. Fresh context, preferably a strong Claude model, explicitly hostile to the implementation assumptions. Different model family is actually valuable here because correlated reasoning errors matter much more during review than during grep.

So rather than:

`Claude → Codex → Claude → Codex → Claude tests/review`

I'd gravitate toward:

`cheap scout → Codex placement judgment → cheap fresh recon/falsifier → Codex implementation → deterministic gate → fresh strong Claude review → Codex fixes → deterministic final gate`

There is a really nice asymmetry there. **Recon needs recall; review needs judgment.** You are currently paying judgment-model prices/tokens for both.

### I would keep the two-stage placement idea

I would *not* collapse placement alternatives and targeted falsification into one call just to save tokens.

That is one of the best ideas in the whole thing:

> scout plausible boundaries → Codex chooses → fresh process tries to falsify the chosen boundary

That protects you from a subtle failure mode where the recon agent invents an architecture and then spends the rest of its context proving itself right. The independence is worth one extra cheap call.

But I would make those outputs almost brutally small.

Your first placement response could realistically be something on this order conceptually:

```text
outcome
candidates[<=3]
discriminators[<=5]
ranges[<=8]
blockers[<=3]
risk
```

Then targeted recon:

```text
verdict
failed_certificate_clause?
facts[<=8]
ranges[<=12]
wiring[<=5]
commands
blockers[<=3]
```

Everything else should be **Codex's job**.

In particular, I love your placement certificate. That is exactly the sort of compressed semantic artifact worth keeping:

> trigger → producer → owned state → consumer → consequence → proof → insufficient substitute

That tiny structure carries an enormous amount of architectural information. I would lean harder on it and delete information that merely restates it.

### One other subtle efficiency improvement

Your rule that supplemental recon uses another fresh Claude invocation is stronger isolation than you need.

Freshness matters a lot for the **falsification pass** and **adversarial review** because independence has epistemic value there. If implementation simply discovers, “I need to know how `sidebar.js` registers this module,” I would happily continue the targeted-recon session or use a dirt-cheap retrieval worker. There is no meaningful independence benefit to making that agent forget everything and reconstruct the neighborhood.

So I would distinguish:

**fresh-by-design:** placement falsifier, final adversarial reviewer.

**continuation preferred:** supplemental retrieval, test-failure investigation.

That could make a noticeable dent in the enormous cache/read churn.

Overall, though, I think you've built something quite good here. This isn't “multi-agent because multi-agent is cool.” There are very explicit information-lifetime boundaries: supervisor state is tiny and durable, the builder owns semantic understanding for one slice, and repository exploration is disposable. **That is the right conceptual model.**

The next optimization isn't really “use fewer agents.” It's **make each agent return only information whose lifetime justifies crossing the boundary**.

And your metrics system is already good enough that you can empirically test the change. I would watch **provider output tokens per retrieved source line**, **provider output tokens per accepted slice**, number of supplemental calls, and late semantic rejections. Right now that first ratio on the Visual Evidence slice is about **24 Claude output tokens for every source line retrieved**. Recon should be able to get *way* below that without losing architectural accuracy.

