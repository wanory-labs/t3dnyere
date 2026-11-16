//! Minimal MCP stdio tool server: in-memory models only, no filesystem or shell access.
use crate::{MAX_BYTES, Result, parse_document};
use serde_json::{Value, json};
use std::io::{BufRead, Read, Write};

pub fn document_schema() -> Value {
    let point = json!({"type":"array","items":{"type":"number","minimum":-1000000,"maximum":1000000},"minItems":3,"maxItems":3});
    let size = json!({"type":"number","minimum":0.0001,"maximum":1000000});
    let id = json!({"type":"string","pattern":"^[A-Za-z0-9_-]{1,64}$"});
    let mut variants = vec![];
    for op in ["sphere", "box", "union", "intersection", "difference"] {
        let mut properties = json!({"id":id,"op":{"const":op}});
        let mut required = vec!["id", "op"];
        match op {
            "sphere" => {
                properties["center"] = point.clone();
                properties["radius"] = size.clone();
                required.extend(["center", "radius"]);
            }
            "box" => {
                properties["center"] = point.clone();
                properties["half_size"] =
                    json!({"type":"array","items":size,"minItems":3,"maxItems":3});
                required.extend(["center", "half_size"]);
            }
            _ => {
                properties["a"] = id.clone();
                properties["b"] = id.clone();
                required.extend(["a", "b"]);
            }
        }
        variants.push(json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}));
    }
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":"T3D procedural model v1","type":"object","additionalProperties":false,
        "required":["version","root","nodes"],"properties":{"version":{"const":1},"root":id,"nodes":{"type":"array","minItems":1,"maxItems":4096,"items":{"oneOf":variants}}}})
}

fn tools() -> Value {
    let mut schema = document_schema();
    schema.as_object_mut().unwrap().remove("$schema");
    let base = json!({"type":"object","required":["document"],"additionalProperties":false,"properties":{"document":schema}});
    let mut sample = base.clone();
    sample["required"] = json!(["document", "point"]);
    sample["properties"]["point"] = json!({"type":"array","minItems":3,"maxItems":3,"items":{"type":"number","minimum":-1000000,"maximum":1000000}});
    json!({"tools":[
        {"name":"validate_model","description":"Validate a procedural solid DAG and report runtime storage size. No filesystem access.","inputSchema":base,"annotations":{"readOnlyHint":true,"openWorldHint":false}},
        {"name":"sample_model","description":"Evaluate signed implicit field at a point in meters. Negative means inside; not generally an exact distance.","inputSchema":sample,"annotations":{"readOnlyHint":true,"openWorldHint":false}},
        {"name":"compile_model","description":"Compile to deterministic T3D v1 bytes, returned as lowercase hex. No filesystem access.","inputSchema":base,"annotations":{"readOnlyHint":true,"openWorldHint":false}}
    ]})
}

fn call(params: &Value) -> Result<Value> {
    let name = params["name"].as_str().ok_or("missing tool name")?;
    if !["validate_model", "sample_model", "compile_model"].contains(&name) {
        return Err("unknown tool".into());
    }
    let args = params["arguments"]
        .as_object()
        .ok_or("arguments must be an object")?;
    for key in args.keys() {
        if key != "document" && !(name == "sample_model" && key == "point") {
            return Err(format!("unknown argument: {key}"));
        }
    }
    let doc = args.get("document").ok_or("missing document")?;
    let model = parse_document(&serde_json::to_vec(doc).map_err(|e| e.to_string())?)?.compile()?;
    let mut out = json!({"valid":true,"instructions":model.instruction_count(),"runtime_bytes":model.to_bytes().len()});
    match name {
        "sample_model" => {
            let point: [f32; 3] =
                serde_json::from_value(args.get("point").cloned().ok_or("missing point")?)
                    .map_err(|e| e.to_string())?;
            let field = model.sample(point)?;
            out["field"] = json!(field);
            out["inside"] = json!(field < 0.0);
        }
        "compile_model" => {
            use std::fmt::Write;
            let mut hex = String::new();
            for byte in model.to_bytes() {
                write!(hex, "{byte:02x}").unwrap();
            }
            out["encoding"] = json!("hex");
            out["asset"] = json!(hex);
        }
        _ => {}
    }
    Ok(out)
}

fn valid_call(params: &Value) -> bool {
    let Some(name) = params["name"].as_str() else {
        return false;
    };
    if !["validate_model", "sample_model", "compile_model"].contains(&name) {
        return false;
    }
    let Some(args) = params["arguments"].as_object() else {
        return false;
    };
    if args
        .keys()
        .any(|key| key != "document" && !(name == "sample_model" && key == "point"))
    {
        return false;
    }
    let Some(doc) = args.get("document") else {
        return false;
    };
    if parse_document(&serde_json::to_vec(doc).unwrap()).is_err() {
        return false;
    }
    if name == "sample_model" {
        let Some(point) = args.get("point") else {
            return false;
        };
        let Ok(point) = serde_json::from_value::<[f32; 3]>(point.clone()) else {
            return false;
        };
        if !crate::point_valid(point) {
            return false;
        }
    }
    true
}

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

/// Newline framed UTF-8 JSON-RPC. EOF exits cleanly; oversized lines terminate.
/// Transport is intentionally local stdio; there is no listener or network service.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> Result<()> {
    let mut initialized = false;
    let mut ready = false;
    loop {
        let mut bytes = Vec::new();
        let read = input
            .by_ref()
            .take(MAX_BYTES + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|e| e.to_string())?;
        if read == 0 {
            return Ok(());
        }
        if bytes.len() as u64 > MAX_BYTES {
            return Err("MCP message exceeds 2 MiB; closing transport".into());
        }
        let response = match serde_json::from_slice::<Value>(&bytes) {
            Err(_) => Some(error(Value::Null, -32700, "invalid JSON")),
            Ok(request) => {
                let id = request.get("id").cloned();
                let valid_id = id
                    .as_ref()
                    .is_none_or(|v| v.is_string() || v.is_i64() || v.is_u64());
                if !request.is_object()
                    || request["jsonrpc"] != "2.0"
                    || !request["method"].is_string()
                    || !valid_id
                {
                    Some(error(Value::Null, -32600, "invalid JSON-RPC request"))
                } else if id.is_none() {
                    if request["method"] == "notifications/initialized" && initialized {
                        ready = true;
                    }
                    None
                } else {
                    let id = id.unwrap();
                    let method = request["method"].as_str().unwrap();
                    let params = &request["params"];
                    let result = match method {
                        "initialize" if !initialized => {
                            if !params["protocolVersion"].is_string()
                                || !params["capabilities"].is_object()
                                || !params["clientInfo"].is_object()
                            {
                                Err((
                                    -32602,
                                    "initialize requires protocolVersion, capabilities, and clientInfo",
                                ))
                            } else {
                                initialized = true;
                                let version = params["protocolVersion"].as_str().unwrap();
                                let version =
                                    if ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"]
                                        .contains(&version)
                                    {
                                        version
                                    } else {
                                        "2025-11-25"
                                    };
                                Ok(
                                    json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"t3dnyere","version":env!("CARGO_PKG_VERSION")}}),
                                )
                            }
                        }
                        "ping" => Ok(json!({})),
                        _ if !ready => Err((-32002, "complete initialize handshake first")),
                        "tools/list" => Ok(tools()),
                        "tools/call" if !valid_call(params) => {
                            Err((-32602, "unknown tool or invalid tool arguments"))
                        }
                        "tools/call" => Ok(match call(params) {
                            Ok(value) => {
                                json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                            }
                            Err(message) => {
                                json!({"content":[{"type":"text","text":message}],"isError":true})
                            }
                        }),
                        _ => Err((-32601, "method not found")),
                    };
                    Some(match result {
                        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
                        Err((code, msg)) => error(id, code, msg),
                    })
                }
            }
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut output, &response).map_err(|e| e.to_string())?;
            output.write_all(b"\n").map_err(|e| e.to_string())?;
            output.flush().map_err(|e| e.to_string())?;
        }
    }
}
