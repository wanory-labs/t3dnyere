# Agent instructions

This is a Rust procedural solid modeling kernel, not a web app. Preserve the headless library/CLI/MCP architecture. Do not introduce an LLM API dependency or arbitrary script evaluation into model documents.

## Navigation

- Overview and honest limits: [README](<README.md>).
- Source graph and evaluator: [lib.rs](<src/lib.rs>).
- Custom binary ABI: [codec.rs](<src/codec.rs>) and [format spec](<docs/format.md>).
- AI integration: [guide](<docs/agents.md>) and [MCP server](<src/protocol.rs>).
- Example asset source: [bracket](<examples/bracket.t3d.json>).
- Representation rationale and external sources: [research](<docs/research.md>).

## Checks

```sh
cargo fmt --all --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked --release --example measure
```

Use `t3dnyere schema` for the machine-readable authoring contract. Adding a node operation must update source deserialization, JSON Schema, compiler, evaluator, codec, docs, and tests together. Do not change v1 opcode meanings or layout silently: introduce a new version and reject unsupported formats. Preserve explicit little-endian serialization and strict decoder bounds. No unsafe decoding or assumptions about Rust struct layout.

## Model edits

1. Read the source document; identify node IDs and root.
2. Change only requested nodes/parameters. IDs are editing identities, not array indices.
3. Validate the complete graph, then query relevant known points.
4. Compile to a new path; CLI deliberately refuses overwrite.
5. Preview if meaningful, inspect diffs, and report geometric limits.

Meters, right-handed coordinates, Y up. Box extents are half sizes. `field < 0` means interior, not exact penetration depth. Boolean fields may retain coincident zero sheets. Never market this as a CAD-exact or manifold-safe kernel. f32 bounds are not absolute accuracy guarantees.

Do not equate custom format with DRM. Do not claim compression/latency/GPU wins without a measured fair baseline. Keep source/provenance separate from runtime assets. Preserve graph/input/work limits and untrusted-input tests; use iterative graph handling to avoid deep-DAG stack overflow.

MCP tools stay stateless/in-memory and cannot access files or shell. Filesystem permission belongs to the calling harness, not the model. Do not add network listeners, credentials, external executors or mutable global sessions without an explicit design and authorization.
