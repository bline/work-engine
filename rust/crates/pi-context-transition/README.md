Pi 1.5.0 context transition adapter
=================================

This standalone package uses the existing Rust native allocation, turn grants,
immutable ingress and transition owner. `PiAdapter` uses `NativeStdio` process
custody. Replacement consumes a store-minted unbound successor allocation
permit. Reopening an unbound allocation is uncertainty, never a reset retry.

The selected profile disables all tools and live providers. The owner validates
one request and one successful response, ordered original Pi events, the actual
provider-input projection, complete session messages and a linked session branch.
The profile and protocol digests select this strict proof contract; the SDK index
digest binds the reserved implementation. Original producer bytes remain Pi.
The existing Codex validator and shared settlement commit remain in the owner.

`fixture.mjs` is SDK-required glue with a labelled fake stream function; it cannot
grant lifecycle admission. The Rust qualification binary installs disposable
fixture-root grants, independently binds checkpoint content, performs one actual
`AgentSessionRuntime.newSession`, rehydrates that exact checkpoint, and reads back
the durable owner after reopening. This is local compatibility evidence, with no
claim of live semantic adequacy or task-owner human-interaction acceptance.

Qualification requires a private source/dependency closure and the pinned Node
image, never a live registry install. The closure manifest, exact custody plans,
Cargo-emitted executable and copied executable digests must be retained outside
this source tree. Run `qualification-gate MANIFEST` before build and fixture
entry. `cargo test --manifest-path ../lifecycle-store/Cargo.toml --test
pi_settlement --offline --locked` exercises protocol refusal cases; the Docker
binary exercises real SDK replacement and owner reconciliation. The outer
`custody.py` plans own build/test/container wait, status and removal.
