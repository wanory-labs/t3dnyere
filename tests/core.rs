use serde_json::{Value, json};
use t3dnyere::{
    Document, Model, Node, Shape, parse_document, protocol,
    render::{self, Camera},
};

fn sphere() -> Document {
    Document {
        version: 1,
        root: "ball".into(),
        nodes: vec![Node {
            id: "ball".into(),
            shape: Shape::Sphere {
                center: [0.0; 3],
                radius: 1.0,
            },
        }],
    }
}
fn binary(op: &str) -> Document {
    parse_document(
        &serde_json::to_vec(&json!({"version":1,"root":"c","nodes":[
        {"id":"a","op":"sphere","center":[0,0,0],"radius":1},
        {"id":"b","op":"sphere","center":[1,0,0],"radius":1},
        {"id":"c","op":op,"a":"a","b":"b"}]}))
        .unwrap(),
    )
    .unwrap()
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}

#[test]
fn primitive_sign_and_translation() {
    let model = sphere().compile().unwrap();
    near(model.sample([0.0; 3]).unwrap(), -1.0);
    near(model.sample([1.0, 0.0, 0.0]).unwrap(), 0.0);
    near(model.sample([2.0, 0.0, 0.0]).unwrap(), 1.0);
    let mut doc = sphere();
    doc.nodes[0].shape = Shape::Box {
        center: [2.0, 0.0, 0.0],
        half_size: [1.0, 2.0, 3.0],
    };
    let model = doc.compile().unwrap();
    near(model.sample([2.0, 0.0, 0.0]).unwrap(), -1.0);
    near(model.sample([4.0, 0.0, 0.0]).unwrap(), 1.0);
    near(model.sample([4.0, 3.0, 3.0]).unwrap(), 2.0f32.sqrt());
    assert!(model.sample([f32::NAN, 0.0, 0.0]).is_err());
    assert!(model.sample([f32::INFINITY, 0.0, 0.0]).is_err());
}
#[test]
fn boolean_set_signs() {
    for (op, at_left, at_overlap) in [
        ("union", true, true),
        ("intersection", false, true),
        ("difference", true, false),
    ] {
        let model = binary(op).compile().unwrap();
        assert_eq!(model.sample([-0.5, 0.0, 0.0]).unwrap() < 0.0, at_left);
        assert_eq!(model.sample([0.5, 0.0, 0.0]).unwrap() < 0.0, at_overlap);
        assert!(model.sample([3.0, 0.0, 0.0]).unwrap() > 0.0);
    }
}
#[test]
fn json_strict_and_compilation_order_independent() {
    let original = include_bytes!("../examples/bracket.t3d.json");
    let mut doc = parse_document(original).unwrap();
    let bytes = doc.compile().unwrap().to_bytes();
    doc.nodes.reverse();
    assert_eq!(bytes, doc.compile().unwrap().to_bytes());
    let mut value: Value = serde_json::from_slice(original).unwrap();
    value["typo"] = json!(true);
    assert!(parse_document(&serde_json::to_vec(&value).unwrap()).is_err());
    value.as_object_mut().unwrap().remove("typo");
    value["nodes"][0]["typo"] = json!(true);
    assert!(parse_document(&serde_json::to_vec(&value).unwrap()).is_err());
}
#[test]
fn graph_errors_and_numeric_limits() {
    let mut doc = sphere();
    doc.version = 2;
    assert!(doc.compile().is_err());
    doc.version = 1;
    doc.root = "missing".into();
    assert!(doc.compile().is_err());
    doc.root = "ball".into();
    doc.nodes.push(doc.nodes[0].clone());
    assert!(doc.compile().is_err());
    let mut doc = binary("union");
    doc.nodes[0].shape = Shape::Union {
        a: "c".into(),
        b: "b".into(),
    };
    assert!(doc.compile().is_err());
    doc.nodes[0].shape = Shape::Union {
        a: "absent".into(),
        b: "b".into(),
    };
    assert!(doc.compile().is_err());
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY, 1e7] {
        let mut doc = sphere();
        doc.nodes[0].shape = Shape::Sphere {
            center: [0.0; 3],
            radius,
        };
        assert!(doc.compile().is_err());
    }
    let mut doc = sphere();
    doc.nodes[0].id = "bad/id".into();
    assert!(doc.compile().is_err());
    doc.nodes.clear();
    assert!(doc.compile().is_err());
    let mut doc = sphere();
    for i in 0..4096 {
        doc.nodes.push(Node {
            id: format!("n{i}"),
            shape: Shape::Sphere {
                center: [0.0; 3],
                radius: 1.0,
            },
        });
    }
    assert!(doc.compile().is_err());
}
#[test]
fn shared_children_work() {
    let mut doc = sphere();
    doc.nodes.push(Node {
        id: "same".into(),
        shape: Shape::Union {
            a: "ball".into(),
            b: "ball".into(),
        },
    });
    doc.root = "same".into();
    near(doc.compile().unwrap().sample([0.0; 3]).unwrap(), -1.0);
}
#[test]
fn codec_roundtrip_all_ops_grid() {
    for op in ["union", "intersection", "difference"] {
        let model = binary(op).compile().unwrap();
        let bytes = model.to_bytes();
        assert_eq!(&bytes[..12], b"T3DNYERE\x01\0\0\0");
        assert_eq!(bytes.len(), 20 + 32 * 3);
        let decoded = Model::from_bytes(&bytes).unwrap();
        assert_eq!(model, decoded);
        for x in -8..=8 {
            for y in -4..=4 {
                let point = [x as f32 * 0.25, y as f32 * 0.25, 0.2];
                assert_eq!(model.sample(point).unwrap(), decoded.sample(point).unwrap());
            }
        }
    }
    let doc = parse_document(include_bytes!("../examples/bracket.t3d.json")).unwrap();
    let model = doc.compile().unwrap();
    assert_eq!(model, Model::from_bytes(&model.to_bytes()).unwrap());
}
#[test]
fn decoder_rejects_corruption_and_every_truncation() {
    let bytes = binary("union").compile().unwrap().to_bytes();
    for n in 0..bytes.len() {
        assert!(Model::from_bytes(&bytes[..n]).is_err(), "truncation {n}");
    }
    for offset in [0, 8, 12, 16, 20, 20 + 20, 20 + 32 * 2 + 4] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Model::from_bytes(&bad).is_err(), "corruption {offset}");
    }
    let mut bad = bytes.clone();
    bad.extend([0]);
    assert!(Model::from_bytes(&bad).is_err());
    let mut bad = bytes.clone();
    bad[36..40].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
    assert!(Model::from_bytes(&bad).is_err());
    // A deterministic malformed-input sweep: decoder must never panic.
    let mut state = 42u32;
    for len in 0..512 {
        let data: Vec<u8> = (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        assert!(Model::from_bytes(&data).is_err());
    }
}
#[test]
fn preview_is_deterministic_bounded_and_visible() {
    let model = sphere().compile().unwrap();
    let bytes = render::preview(&model, 32, 32, Camera::default()).unwrap();
    let header = b"P6\n32 32\n255\n";
    assert!(bytes.starts_with(header));
    assert_eq!(bytes.len(), header.len() + 32 * 32 * 3);
    assert!(
        bytes[header.len()..]
            .as_chunks::<3>()
            .0
            .iter()
            .any(|p| *p != [19, 25, 34])
    );
    assert_eq!(
        bytes,
        render::preview(&model, 32, 32, Camera::default()).unwrap()
    );
    assert!(render::preview(&model, 513, 10, Camera::default()).is_err());
    assert!(render::preview(&model, 0, 10, Camera::default()).is_err());
    let camera = Camera {
        target: Camera::default().eye,
        ..Camera::default()
    };
    assert!(render::preview(&model, 10, 10, camera).is_err());
}
fn rpc(messages: &[Value]) -> Vec<Value> {
    let input = messages
        .iter()
        .map(|v| format!("{v}\n"))
        .collect::<String>();
    let mut output = vec![];
    protocol::serve(std::io::Cursor::new(input), &mut output).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn init() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})
}
fn ready() -> Value {
    json!({"jsonrpc":"2.0","method":"notifications/initialized"})
}
#[test]
fn mcp_lifecycle_tools_and_errors() {
    let doc = serde_json::to_value(sphere()).unwrap();
    let out = rpc(&[
        json!({"jsonrpc":"2.0","id":0,"method":"tools/list"}),
        init(),
        ready(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"sample_model","arguments":{"document":doc,"point":[0,0,0]}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"compile_model","arguments":{"document":doc}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"sample_model","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"unknown"}),
    ]);
    assert_eq!(out.len(), 7);
    assert_eq!(out[0]["error"]["code"], -32002);
    assert_eq!(out[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(out[2]["result"]["tools"].as_array().unwrap().len(), 3);
    assert_eq!(out[3]["result"]["isError"], false);
    let sampled: Value =
        serde_json::from_str(out[3]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(sampled["field"], -1.0);
    assert_eq!(sampled["inside"], true);
    let compiled: Value =
        serde_json::from_str(out[4]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(compiled["asset"].as_str().unwrap().len(), 104);
    assert_eq!(out[5]["error"]["code"], -32602);
    assert_eq!(out[6]["error"]["code"], -32601);
}
#[test]
fn mcp_distinguishes_bad_arguments_from_model_errors() {
    let mut doc = serde_json::to_value(sphere()).unwrap();
    doc["root"] = json!("missing");
    let out = rpc(&[
        init(),
        ready(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"absent","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"validate_model","arguments":{"document":doc}}}),
    ]);
    assert_eq!(out[1]["error"]["code"], -32602);
    assert_eq!(out[2]["result"]["isError"], true);
}

#[test]
fn csg_bound_is_not_true_distance_and_zero_set_is_not_regularized() {
    let doc = Document {
        version: 1,
        root: "c".into(),
        nodes: vec![
            Node {
                id: "a".into(),
                shape: Shape::Box {
                    center: [0.0, 1.0, 0.0],
                    half_size: [1.0, 10.0, 10.0],
                },
            },
            Node {
                id: "b".into(),
                shape: Shape::Box {
                    center: [1.0, 0.0, 0.0],
                    half_size: [10.0, 1.0, 10.0],
                },
            },
            Node {
                id: "c".into(),
                shape: Shape::Intersection {
                    a: "a".into(),
                    b: "b".into(),
                },
            },
        ],
    };
    near(doc.compile().unwrap().sample([2.0, 2.0, 0.0]).unwrap(), 1.0); // true distance is sqrt(2)
    let mut doc = sphere();
    doc.nodes.push(Node {
        id: "empty_interior".into(),
        shape: Shape::Difference {
            a: "ball".into(),
            b: "ball".into(),
        },
    });
    doc.root = "empty_interior".into();
    let model = doc.compile().unwrap();
    near(model.sample([1.0, 0.0, 0.0]).unwrap(), 0.0); // retained zero sheet
    assert!(model.sample([0.0; 3]).unwrap() > 0.0);
}

#[test]
fn deepest_allowed_dag_needs_no_recursive_evaluation() {
    let mut doc = sphere();
    let mut previous = "ball".to_owned();
    for i in 1..4096 {
        let id = format!("n{i}");
        doc.nodes.push(Node {
            id: id.clone(),
            shape: Shape::Union {
                a: previous.clone(),
                b: "ball".into(),
            },
        });
        previous = id;
    }
    doc.root = previous;
    let model = doc.compile().unwrap();
    near(model.sample([0.0; 3]).unwrap(), -1.0);
    assert_eq!(model, Model::from_bytes(&model.to_bytes()).unwrap());
    assert!(render::preview(&model, 16, 16, Camera::default()).is_err());
}

#[test]
fn mcp_framing_and_message_limits() {
    let mut out = vec![];
    protocol::serve(std::io::Cursor::new(b"not-json\n[]\n"), &mut out).unwrap();
    let values: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(values[0]["error"]["code"], -32700);
    assert_eq!(values[1]["error"]["code"], -32600);
    assert!(
        protocol::serve(
            std::io::Cursor::new(vec![b'x'; 2 * 1024 * 1024 + 1]),
            &mut vec![]
        )
        .is_err()
    );
}
