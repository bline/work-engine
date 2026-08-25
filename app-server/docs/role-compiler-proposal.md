# Skill Compiler: Structured Skill Sources and Generated Model-Facing Projections

**Status:** Concept proposal  
**Initial experiment:** One-time legacy migration of `skills/slice-builder/SKILL.md`  
**Decision state:** Architecture candidate; canonical ownership has not migrated

## Summary

Work Engine skills are currently expressed primarily as `SKILL.md` instructions. For role-bearing skills, related authority, environment, invariant, capability, and interface facts are also distributed across shared catalogs, environment definitions, runtime configuration, tests, and role-specific code.

This proposal introduces a provider-neutral **skill compiler**. A skill becomes a structured authored object whose model-facing instructions, runtime requirements, validators, tests, environment graphs, and documentation are generated projections. Role semantics are an optional structural profile for skills that actually operate as roles; procedures, capabilities, adapters, and validators are not forced into a role ontology.

The target ownership model is:

```text
Authored sources
├── structure.yaml     semantic skill source and optional role profile
├── interface.yaml     enforceable boundary
└── shared catalogs    referenced global truth
          ↓
Provider-neutral skill compiler
          ↓
Generated projections
├── SKILL.md
├── runtime requirements
├── validators and tests
├── environment projections and graphs
└── documentation
```

The skill remains a manipulable semantic object. Domain judgment remains in canonical semantic content and is faithfully delivered through model-facing instructions. Code implements generic compilation and enforcement machinery rather than a separate behavioral path for every role.

Normal operation is strictly forward: canonical decomposed sources produce `SKILL.md`. The compiler does not recover structure from generated prose, and direct edits to generated `SKILL.md` are drift.

The first experiment must be deliberately sterile: use a bounded, one-time migration process to decompose the legacy slice-builder skill without improving, simplifying, or reinterpreting it; regenerate it byte-for-byte through the forward compiler; preserve its environment relations; and migrate ownership only after the round trip succeeds.

## Motivation

A prose skill is easy for humans and models to read, but its internal structure is difficult to query or transform mechanically. Important facts may be repeated across prose, environment declarations, runtime manifests, validators, and tests. A role-specific runtime gateway can mechanically enforce some boundaries, but one code path per role disperses role meaning across implementation artifacts and makes role generation, comparison, analysis, and revision more difficult.

Authoring a skill as structured sources enables:

- stable identity for semantic sections and boundaries;
- explicit invariant, authority, capability, ownership, state, and artifact relations;
- structural comparison across skills, roles, and revisions;
- generation of exact model-facing instructions;
- reusable, generic runtime enforcement;
- generation of validators and contract tests;
- derivation of environment views and documentation;
- detection of contradictions expressible in a closed structural vocabulary;
- construction of skill and role editors and creation workflows;
- controlled skill and role variants without copying entire skills; and
- eventual context selection based on proven instruction closure rather than ad hoc prompt trimming.

This is more than Markdown templating. It is compilation from canonical skill sources into multiple purpose-specific projections.

## Architectural principles

### 1. The structured skill package is canonical

After an explicit ownership migration, `structure.yaml` becomes the canonical semantic source for the skill and its optional role profile. `interface.yaml` becomes the canonical source for its mechanically enforceable boundary. Exact authored instruction content remains owned in decomposed source fields or referenced content blocks; `SKILL.md` becomes a generated model-facing artifact rather than an editable owner.

Before that migration, the existing skill and environment sources remain canonical. The compiler experiment does not change ownership merely by producing a successful draft.

### 2. Structure and interface have different jobs

`structure.yaml` describes the skill's semantic content, dependencies, and references. When the skill carries a role profile, it also describes the valid semantic world around that role:

- objective and identity;
- judgment instructions;
- authority and prohibitions;
- invariant membership and causal meaning;
- capabilities the role may invoke;
- state and artifacts it may observe, mutate, own, consume, or emit;
- mediated transitions;
- relationships to other roles; and
- context and continuity obligations intrinsic to the role.

`interface.yaml` declares the bounded subset of that world that generic runtime machinery can enforce:

- accepted request and response shapes;
- exact, digest-bound, and freshness bindings;
- effect ceilings;
- mechanically enforced capability restrictions;
- deterministic normalizations; and
- named protocol boundaries.

The interface does not redefine the skill's meaning. It references semantic boundaries owned by the structure and maps them to a closed enforcement vocabulary.

### 3. Compilation is forward-only

The steady-state authoring path is:

```text
canonical decomposed sources → complete skill projection → generated SKILL.md
```

The reverse path is not part of compilation. Existing prose-owned skills may pass through a bounded, one-time assisted migration, but accepted structured sources are never refreshed by re-inferring meaning from generated prose. After ownership migration, direct edits to `SKILL.md` fail generated-artifact drift checks.

### 4. Runtime realization remains a deployment decision

The compiler may emit runtime requirements, capability ceilings, and environment constraints. The runtime manifest still owns provider and deployment choices such as:

- model and service tier;
- provider adapter;
- sandbox realization;
- actual capabilities granted;
- working directory;
- approval policy; and
- retained or ephemeral thread binding.

A runtime may narrow a role's permitted effects but may not silently exceed its declared effect ceiling.

### 5. Semantic authority and effect enforcement remain distinct

The canonical skill structure owns what an authority statement means. For example, the strategic planner's canonical instructions own the meaning of “advisory,” and the generated skill delivers that meaning to the model. Runtime machinery cannot prove that natural-language output never overstates authority.

The runtime can enforce the corresponding effect boundary: it can deny campaign mutation, repository writes, publication, or other unauthorized transitions.

```text
Canonical structure: owns semantic meaning
Generated skill:     delivers model-facing meaning
Runtime:             enforces mechanically expressible effects
```

### 6. Domain judgment remains in authored semantics and model execution

Skill- or role-specific judgment must not migrate into deterministic compiler or runtime code. Semantic classification during legacy migration is a reviewed model judgment, not a compiler function. Custom code may attach to a named boundary only when a deterministic normalization, external protocol, or effect cannot be expressed through the bounded generic contract.

Code does not become an alternative definition of the skill or role.

### 7. The structural language remains deliberately bounded

Fields such as `applies_when`, `requires`, and `bindings` are the beginnings of a language. The initial compiler must keep them declarative and closed.

- Conditions initially annotate rendered instructions; they do not dynamically omit instructions.
- Bindings use named primitives such as `exact`, `digest_bound`, and `fresh_reference`.
- No arbitrary expressions, callbacks, embedded scripts, or general conditional language are permitted.
- No inheritance mechanism is introduced during the bootstrap.
- Natural-language contradictions remain subject to semantic review.

Conditional prompt loading is a later capability. It requires a closure contract proving that no instruction necessary for correct execution was excluded.

## Authored skill package

The target slice-builder package is:

```text
skills/slice-builder/
├── structure.yaml
├── interface.yaml
├── SKILL.md              # generated
└── references/           # existing referenced material where applicable
```

### `structure.yaml`

The initial structure should be only as expressive as required to losslessly represent the existing skill. Prose remains authored text in YAML block scalars; the compiler does not synthesize or paraphrase it.

Illustrative shape:

```yaml
schema_version: 1
skill_id: slice-builder
label: Slice Builder

sections:
  - id: identity
    kind: semantic
    heading:
      level: 1
      text: Slice Builder
    content: |-
      Exact existing prose.

  - id: authority-boundary
    kind: authority
    heading:
      level: 2
      text: Authority boundary
    bound_by:
      - INV-001
      - INV-002
    boundaries:
      - mutation.accepted-scope
    content: |-
      Exact existing prose.

boundaries:
  - id: mutation.accepted-scope
    kind: authority
    effect: repository_write
    condition: |-
      Repository mutation is limited to accepted task paths after
      plan acceptance.

profiles:
  role:
    role_id: role.builder
    relations:
      may_invoke:
        - capability.repository_evidence
        - capability.repository_mutation
      owns:
        - artifact.plan
        - artifact.implementation_receipt
      mediated_transitions:
        - transition: plan_acceptance
          mediated_by: role.supervisor
      forbidden_from:
        - accepting_own_slice
```

The exact bootstrap schema must be discovered from the existing skill rather than designed speculatively. The example establishes categories, not a final schema. A general skill need not declare role-only fields; the eventual schema must distinguish common instruction-package structure from an optional role profile.

### `interface.yaml`

The interface remains smaller and less expressive than the semantic structure.

Illustrative shape:

```yaml
schema_version: 1

requests:
  slice:
    schema: accepted-slice-v1

responses:
  plan:
    schema: slice-plan-v1
  handoff:
    schema: builder-handoff-v1

bindings:
  accepted_scope: digest_bound
  campaign_revision: exact

effects:
  repository_write:
    semantic_boundary: mutation.accepted-scope
    enforcement: scoped_write
  slice_acceptance:
    enforcement: denied
  publication:
    enforcement: denied
```

Every `semantic_boundary` must resolve to a compatible boundary in `structure.yaml`. The interface may expose mechanical enforcement for a semantic boundary, but it may not silently invent or broaden that boundary.

### Shared catalogs

Global catalogs own shared invariant, mechanism, capability, state, artifact, and relation truth. Skill structure references stable identities in those catalogs when they apply.

An invariant ID is provenance, not sufficient model instruction. A generated skill must contain the causal meaning required by the role; it cannot merely tell the model that it is “bound by INV-007.”

For a role-bearing skill, the compiler therefore resolves selected catalog content into the complete skill projection. A projection rule may select fields such as:

```yaml
invariant_projection:
  include:
    - condition
    - causal_parent
    - role_implication
```

If the necessary role-specific implication does not exist in a shared catalog, it must be explicitly authored in the role structure. The compiler must not fabricate semantic instructions.

## Compiler architecture

The compiler core is deterministic, forward-only, and provider-neutral:

```text
Parse authored sources
        ↓
Validate source schemas
        ↓
Resolve catalog references
        ↓
Construct complete skill IR
        ↓
Validate structural closure
        ↓
Render projections
```

### Skill intermediate representation

The skill intermediate representation is a generated, normalized object. It is not a fourth authored source.

It provides one complete input to renderers and validators:

```text
structure.yaml ─┐
interface.yaml ─┼──→ complete skill IR
catalogs ───────┘          ├──→ Codex skill renderer
                           ├──→ runtime-requirements renderer
                           ├──→ validator/test generator
                           ├──→ environment projection
                           └──→ documentation renderers
```

The skill IR should preserve source identity and provenance for every resolved element. Generated artifacts should have provenance sufficient to detect stale projections. When byte identity with a legacy `SKILL.md` is required, build provenance belongs in the IR, generation manifest, or receipt rather than being injected into the model-facing bytes.

### Codex skill renderer

The Codex renderer owns provider-specific skill formatting:

- YAML frontmatter;
- heading levels and ordering;
- prose and list formatting;
- blank-line policy;
- code fences;
- reference rendering;
- line endings; and
- final-newline behavior.

Provider formatting does not become part of the underlying skill semantics or optional role profile.

The first renderer must reproduce the pinned slice-builder skill byte-for-byte. Exact byte equality is mechanically provable. Semantic fidelity beyond equality remains a reviewed judgment.

### Runtime-requirements renderer

The compiler may emit requirements such as:

- required observable states;
- required capabilities;
- effect ceilings;
- prohibited effects;
- necessary mediated transitions; and
- context-lifetime constraints intrinsic to correct continuation.

These requirements constrain runtime configuration but do not silently choose its provider realization.

### Validator and test generation

The compiler may generate tests only for claims expressible through the closed schema. Examples include:

- unresolved catalog references;
- duplicate stable identities;
- unsupported binding primitives;
- an interface effect that lacks a semantic boundary;
- a runtime grant exceeding the role's effect ceiling;
- an owned artifact with no declared lifecycle path;
- structurally incompatible role relations; and
- stale generated projections.

Contradictions between natural-language instructions remain subject to instruction and semantic review.

## Relationship to environment projections

The existing Agent Environment Graph implementation already owns substantial structural machinery needed by the compiler. Its canonical inputs are `docs/workflow-invariants.md` and `docs/agent-environments.yaml`; its deterministic CLI parses and validates the catalogs and role vocabulary, resolves referenced closure, records source hashes, renders role-scoped projections, and detects drift. The generated `docs/agent-environment-views/slice-builder.yaml` demonstrates the resulting role data. It records relations including:

- `bound_by`;
- `may_invoke`;
- `may_observe`;
- `may_mutate`;
- `owns`;
- `consumes`;
- `emits`;
- `mediated_transitions`; and
- `forbidden_from`.

It also expands the relevant invariant and mechanism closure. That expanded projection is useful for analysis, validation, graph generation, and review, but it should not become the manually authored per-role source.

The canonical `structure.yaml` should remain skill-local and compact. For a role-bearing skill, the compiler joins its role profile with shared catalogs to reproduce the expanded environment view. The implementation should reuse a shared structural core or consume the Agent Environment Graph as a validation and projection backend; it must not create a second owner for relation semantics merely to avoid the current CLI boundary. The exact code-sharing route should be chosen from implementation evidence.

Before ownership migration, the central environment definition remains canonical and the generated builder view is the comparison oracle. After an accepted migration, the central environment definition may itself become an aggregate projection rather than a competing canonical declaration.

## Structural invariants

The skill compiler must preserve the following invariants:

1. **Single semantic ownership.** Every semantic assertion has one canonical authored owner.
2. **Single boundary ownership.** Every mechanically enforceable rule has one canonical interface declaration linked to its semantic boundary where applicable.
3. **No authority by projection.** A generated artifact, imported classification, or structural relation cannot independently grant authority. Every authority-bearing relation must resolve to an accepted authority source.
4. **No runtime expansion.** A runtime may narrow a role's effects but may not exceed its effect ceiling.
5. **No fabricated semantics.** The compiler does not invent, paraphrase, or infer missing skill or role instructions.
6. **Complete model projection.** Stable IDs and references are resolved into the causal instructions the model needs for correct execution.
7. **Deterministic generation.** An identical complete input closure—including structure, interface, referenced catalogs and content, renderer configuration, and compiler version—produces identical generated bytes.
8. **Idempotent generation.** Regenerating an unchanged skill package produces no changes.
9. **Explicit provenance.** Generation records identify source revisions or digests and compiler identity without requiring provenance text inside byte-constrained model-facing artifacts.
10. **Fail-closed resolution.** Missing, stale, conflicting, or unsupported structural inputs prevent publication of the projection.
11. **Bounded language.** The initial schema contains no arbitrary execution or hidden general-purpose expression language.
12. **Reviewed semantic fidelity.** Mechanical equality and structural validation do not replace instruction review.
13. **Provider-neutral meaning.** Provider renderers realize a skill but do not own its underlying semantics.
14. **No premature conditional omission.** Applicability metadata annotates instructions until a later closure contract authorizes selective loading.
15. **Forward-only ownership.** After migration, generated prose never becomes an editable or reverse-parsed semantic owner.

## Bootstrap experiment: one-time slice-builder migration

### Objective

Prove that the existing slice-builder skill can be migrated into structurally meaningful skill source with a role profile and regenerated exactly without changing its behavior, environment, authority, or established validation.

The experiment contains two distinct boundaries: a one-time, judgment-bearing migration that produces reviewed experimental sources, and a deterministic forward compiler that consumes those sources. It proves the compiler before changing canonical ownership; it does not make legacy decomposition a compiler responsibility.

### Non-goals

The bootstrap does not:

- improve or shorten the slice-builder instructions;
- introduce role variants;
- infer new structural facts;
- change the existing role environment;
- replace existing validation;
- add conditional prompt loading;
- generalize every role construct;
- create a steady-state reverse parser for `SKILL.md`;
- migrate other roles; or
- replace the current canonical skill before acceptance.

### Procedure

1. Pin the exact current `skills/slice-builder/SKILL.md` bytes, repository revision, and digest.
2. Pin every current canonical source contributing to the builder environment.
3. Inventory the skill's frontmatter, headings, ordering, prose, lists, code fences, references, whitespace, line endings, and final newline.
4. Through a bounded one-time migration process, decompose every element into experimental `structure.yaml` using stable identities and the narrowest schema capable of representing it.
5. Preserve exact source spans and producer attribution for semantic classifications. Treat inferred invariant, authority, ownership, capability, and equivalence mappings as candidates until reviewed.
6. Require every authority-bearing relation to resolve to an accepted canonical source; an unresolved candidate cannot grant authority merely by appearing in the experimental structure.
7. Create experimental `interface.yaml` only for boundaries already mechanically expressed or required by the current role contract. Do not invent new enforcement.
8. Review the proposed decomposition and boundary mappings as experimental compiler inputs without migrating canonical ownership.
9. Resolve shared invariant and environment references without replacing model-relevant causal instructions with opaque IDs.
10. Construct the complete provider-neutral skill IR through the deterministic compiler.
11. Render `SKILL.md` through the Codex renderer.
12. Require byte-for-byte equality with the pinned original.
13. Regenerate a second time and require no file changes.
14. Generate the expanded builder environment through the existing structural machinery and require no relation or invariant delta.
15. Run all existing skill, campaign, structural, and instruction-review checks without weakening or bypassing them.
16. Review the decomposition for semantic issues not established by byte equality.
17. Record the experiment and its evidence outside the byte-identical generated skill.
18. Perform an explicit ownership migration only after the result is accepted by the existing decision authority.

### Meaningful-decomposition constraint

Byte-identical output is necessary but not sufficient evidence of useful structure. The bootstrap must not succeed by storing the entire original skill in a single opaque field.

Each existing semantic section must receive a stable identity and appropriate structural kind. Exact prose may remain in block scalars or referenced canonical content blocks, but the schema must expose the distinctions needed by the skill and its role profile. Whole-document passthrough is prohibited.

This makes the experiment stronger than “YAML can reproduce Markdown.” It tests whether the skill and its role profile can be represented as structured source without losing the exact executable projection.

### Acceptance criteria

The bootstrap is successful only when:

- the generated skill is byte-identical to the pinned original;
- generation is deterministic and idempotent;
- no opaque whole-document escape hatch was used;
- every source and resolved reference has traceable provenance;
- every authority-bearing relation resolves to an accepted authority source;
- all model-required invariant meaning remains visible in the generated skill;
- the expanded environment contains no unexplained relation or invariant delta;
- the interface does not invent or broaden semantic authority;
- runtime requirements do not silently select deployment choices;
- all existing checks pass unchanged;
- instruction review finds no semantic regression; and
- ownership migration is separately authorized and recorded.

### Steady-state operation after migration

After accepted ownership migration:

```text
structure.yaml + interface.yaml + catalogs
                    ↓
          deterministic skill compiler
                    ↓
      SKILL.md + other generated projections
```

Authors, editors, generators, reviewers, and automation operate on the decomposed canonical sources. `SKILL.md` is assembled for Codex consumption and may be deleted and reproduced. The compiler never uses it to refresh canonical meaning. A direct edit is detected as generated-artifact drift and must be reapplied to its semantic owner rather than preserved in the generated file.

## Ownership migration

Today, the role skill and central environment sources are canonical inputs. Successful generation alone does not reverse that ownership.

The migration should be an explicit transition:

```text
Before acceptance
  SKILL.md + central environment sources → experimental decomposed sources

After accepted migration
  structure.yaml + interface.yaml + catalogs → generated SKILL.md and environment projections
```

The migration record should identify:

- the old canonical sources and pinned revisions;
- the new canonical sources;
- generated artifacts and their consumers;
- the compiler version and source digests;
- validation and review evidence;
- the authority accepting the transition; and
- rollback or reconstruction expectations.

Until that transition is accepted, generated files remain experimental projections and cannot silently supersede existing authority.

## Implications for App Server role code

The current strategic-planner runtime modules are useful discovery artifacts. They revealed reusable boundary concepts including:

- exact request binding;
- evidence-cutoff validation;
- completion handling;
- durable logical-role-to-thread binding;
- idempotent delivery;
- response-shape validation; and
- effect denial.

These should become bounded generic contract and runtime primitives before another role-specific gateway is implemented.

Under the skill-compiler model for a role-bearing skill:

```text
Skill and role meaning     → structure.yaml
Mechanical boundary        → interface.yaml
Model-facing instructions  → generated SKILL.md
Deployment realization     → runtime manifest
Provider thread identity   → runtime binding registry
Boundary enforcement       → generic runtime
```

Role-specific code remains an escape hatch for named deterministic integrations that cannot be represented by the bounded generic contract. It does not become the default realization of a role.

## Deferred questions

The bootstrap should not decide these prematurely:

- whether structure files should support composition or inheritance;
- how role variants are represented;
- how canonical content blocks may be shared across skills without obscuring ownership;
- which instruction-package kinds beyond the initial role profile need explicit structural profiles;
- which structural relations are intrinsic to a role versus environment-specific;
- how runtime manifests prove satisfaction of emitted requirements;
- how interface schemas and validation primitives are registered and versioned;
- whether generated skill IR is persisted or remains ephemeral;
- how semantic revisions affect retained runtime bindings;
- how selective instruction loading proves semantic closure; and
- when the central environment definition should become a generated aggregate.

The slice-builder decomposition should provide evidence for these decisions rather than forcing the first schema to predict them.

## Proposed next decision

Authorize only the sterile slice-builder migration and compiler experiment:

> Build the smallest deterministic, provider-neutral skill compiler capable of assembling reviewed experimental slice-builder sources into its existing `SKILL.md` byte-for-byte, while preserving its expanded environment and all existing validation. Create those experimental sources through a separate bounded, one-time legacy migration with evidence-bearing semantic judgments. Do not make reverse decomposition a compiler feature, migrate canonical ownership, or implement a second skill until the experiment is reviewed and accepted.

This experiment establishes whether decomposed skill sources can preserve the exact model-facing artifact and established environment meaning. If successful, it provides an evidence-backed foundation for skill and role generation, analysis, runtime projection, and automation without creating a handcrafted code path for every role.
