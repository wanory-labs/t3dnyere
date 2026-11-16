# T3D wire format v1

Status: prototype. Compatibility means exactly version 1, not a promise to retain this layout forever. Extension `.t3d`; authoring JSON is a separate source representation. No dependency on OBJ, FBX or glTF.

## Header (20 bytes)

All integers and floats are little-endian. Floats are IEEE-754 binary32. Fields are packed explicitly, not a Rust struct memory dump.

| Byte offset | Type | Value |
|---|---|---|
| 0 | 8 bytes | ASCII `T3DNYERE` |
| 8 | u32 | Version = 1 |
| 12 | u32 | Instruction count N, 1..4096 |
| 16 | u32 | Root instruction index, less than N |

Exactly N records follow, each 32 bytes. Total length **20+32N**, no trailer. Too-short and trailing bytes are errors. Maximum valid asset length = 131092 bytes.

## Record (eight u32 words)

Word 0 is opcode. Each f32 value uses its bit representation in the corresponding word. Reserved words must be zero. Zero-based record indices; operation operands must reference records strictly earlier than the current record.

| Opcode | Operation | Words 1..7 |
|---|---|---|
| 1 | sphere | center.x, center.y, center.z, radius, 0, 0, 0 |
| 2 | box | center.x, center.y, center.z, half.x, half.y, half.z, 0 |
| 3 | union | a:u32, b:u32, 0, 0, 0, 0, 0 |
| 4 | intersection | a:u32, b:u32, 0, 0, 0, 0, 0 |
| 5 | difference A minus B | a:u32, b:u32, 0, 0, 0, 0, 0 |

Positions must be finite in ±1e6 meters, dimensions finite in [0.0001, 1e6] meters. These limits constrain input, not f32 accuracy. Unknown opcodes, nonzero reserved fields, unknown versions and invalid indices are rejected before exposing a Model.

No strings, source IDs, materials, compression, checksum or authentication are stored in v1. This is not zero-copy: the loader decodes validated instructions into owned Rust storage. Bit mutations that still produce valid values are not detected as corruption. Do not confuse structural validation with authenticity.

## Evaluation contract

Right-handed coordinate convention, Y up, units meters. For point p:

```text
sphere: length(p-center)-radius
box: q=abs(p-center)-half_size
     length(max(q,0)) + min(max(q.x,q.y,q.z),0)
union: min(field[a],field[b])
intersection: max(field[a],field[b])
difference: max(field[a],-field[b])
```

Compute records in order, return root's field. Negative = inside, positive = outside. Zero is a field boundary, **not necessarily a regularized solid boundary**. Boolean composition is not generally an exact Euclidean distance, and coincident surfaces may leave zero sheets. No epsilon is added to `inside`; preview has its own 0.0005-meter hit tolerance and 0.001-meter gradient step. Those tolerances do not guarantee feature preservation.

## Authoring and determinism

IDs are ASCII letters/digits/underscore/hyphen, 1..64 bytes. Unknown fields, missing references, duplicate IDs, graph cycles, invalid root and version are errors. Array ordering carries no semantics. Compiler uses lexicographically ordered ready IDs in Kahn's topological sort, so reordering the same nodes preserves output bytes. Input values are decoded to f32; JSON numbers may round. ID renaming may change record ordering. Unreachable nodes are retained and validated. No graph recursion is needed.

Encoding and decoding: [codec.rs](<../src/codec.rs>). Authoring/evaluation: [lib.rs](<../src/lib.rs>). Regression cases: [core tests](<../tests/core.rs>).

## Loading in the future custom game engine

Use the same library to avoid two diverging implementations:

```rust
use t3dnyere::Model;

fn load_asset(bytes: &[u8]) -> Result<Model, String> {
    Model::from_bytes(bytes)
}

fn query(model: &Model) -> Result<f32, String> {
    model.sample([0.0, 0.0, 0.0])
}
```

The library does not read files by itself. Caller bounds I/O and owns storage/lifetime. The CLI bounds inputs at 2 MiB. This API supports field queries and the included CPU preview, not integration with a GPU renderer or collision system yet.
