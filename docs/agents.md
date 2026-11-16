# Codex / harness / MCP integration

## Contract

The integration is provider-neutral: subprocess CLI, source JSON, JSON Schema, Rust API, and local MCP stdio. It does not embed an LLM SDK or execute model-generated scripts. An agent edits declarative parameters and operands, not arbitrary executable code.

[Tested] Rust integration tests exercise CLI via subprocess and MCP framing/lifecycle in memory. A separate local subprocess smoke test covers the MCP executable. **Not yet tested inside a real Codex or DSH client**; config examples below are setup instructions, not a verified vendor certification.

## CLI path (works without MCP)

Build the binary first and give your agent access to it:

```sh
cargo build --release --locked
./target/release/t3dnyere schema
./target/release/t3dnyere validate examples/bracket.t3d.json
./target/release/t3dnyere sample examples/bracket.t3d.json 0 0 0
./target/release/t3dnyere compile examples/bracket.t3d.json candidate.t3d
./target/release/t3dnyere preview candidate.t3d candidate.ppm 128
```

Use [the bracket source](<../examples/bracket.t3d.json>) as a template. All normal CLI outputs are one JSON value. Failure: exit 1 and `{"ok":false,"error":"..."}` on stdout. Help is plain text. Compiler output paths must be new. Never replace the source with the biner. Shell access carries the permissions of the harness, so use its own sandbox/approval settings.

Suggested agent instruction:

> Change only node `cutout` to radius 0.9; preserve all IDs and other parameters. Validate the full document, sample points relevant to the intended change, compile to a new asset, then produce a preview. Report validation failures; do not silently delete operands or change the root to make validation pass.

Stable IDs make diffs reviewable; **they are not persistent revision locks**. There is no transactional patch server or concurrent-edit protocol yet. Host agents should serialize edits and review diffs; validate/compile a snapshot before publishing it.

## MCP stdio

```sh
./target/release/t3dnyere mcp
```

MCP uses newline-delimited UTF-8 JSON-RPC on stdin/stdout, not LSP Content-Length framing. No human-readable text is emitted on protocol stdout. On oversized messages the server exits and logs to stderr. It does not open a network port.

Example Codex config fragment (replace the absolute path):

```toml
[mcp_servers.t3dnyere]
command = "/absolute/path/to/t3dnyere/target/release/t3dnyere"
args = ["mcp"]
```

For hosts accepting the common JSON MCP server config shape:

```json
{
  "mcpServers": {
    "t3dnyere": {
      "command": "/absolute/path/to/t3dnyere/target/release/t3dnyere",
      "args": ["mcp"]
    }
  }
}
```

This JSON shape is **not asserted to be DSH's configuration schema**. In another harness, register that same executable and argument through its supported MCP configuration/UI. Use an absolute executable path, not a repo-relative path dependent on the client's cwd. Rebuild/restart after modifying the Rust server. No user configuration is modified automatically by this project.

### Tools

| Tool | Arguments | Result |
|---|---|---|
| `validate_model` | `document` | valid, instruction count, runtime bytes |
| `sample_model` | `document`, `point: [x,y,z]` | above + field, inside |
| `compile_model` | `document` | above + `encoding: "hex"`, asset hex |

`document` is the complete source JSON object, not a file path. Each call is independent and stateless. Compiler returns hex rather than writing a file. Clients decide whether/how to save it, subject to host permission. No tool reads/writes files, fetches URLs, runs a shell, or loads plugins.

Malformed tool arguments/unknown tools yield JSON-RPC `-32602`; geometry/graph validation failures return MCP tool content with `isError: true`. Success has `isError: false`; content is text containing a JSON object. Full input schemas are returned by `tools/list`; the CLI `schema` command returns the authoring schema alone. Semantic checks such as graph cycles still require compilation.

The small server supports initialize, initialized notification, ping, tools/list and tools/call. It advertises **only tools**, no resources/prompts/tasks. It negotiates 2025-11-25, 2025-06-18, 2025-03-26, or 2024-11-05; unknown versions receive the latest supported version. Client may disconnect if it cannot support that version. Pagination is unnecessary for the three static tools. It is not a general MCP framework or exhaustive conformance implementation.

Manual handshake (one compact JSON message per line):

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"manual","version":"1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"sample_model","arguments":{"document":{"version":1,"root":"ball","nodes":[{"id":"ball","op":"sphere","center":[0,0,0],"radius":1}]},"point":[0,0,0]}}}
```

Expected final tool result includes `field: -1.0` and `inside: true`. EOF shuts down the process. Maximum message = 2 MiB including newline; maximum graph = 4096 nodes. There is no authentication over stdio: the launching host is the trust boundary. Apply OS resource controls for untrusted workloads; bounds are not a complete denial-of-service defense.

References: [MCP stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [MCP tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools). Implementation: [protocol.rs](<../src/protocol.rs>).
