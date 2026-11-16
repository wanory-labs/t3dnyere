//! Headless procedural solid modeling. All coordinates are meters, Y up.
pub mod codec;
pub mod protocol;
pub mod render;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, String>;
pub const MAX_NODES: usize = 4096;
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const LIMIT: f32 = 1_000_000.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub root: String,
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Node {
    pub id: String,
    #[serde(flatten)]
    pub shape: Shape,
}

// Serde's flatten + deny_unknown_fields combination is unsupported. Decode an
// explicit wire enum to retain strict unknown/duplicate-field validation.
impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Sphere {
                id: String,
                center: [f32; 3],
                radius: f32,
            },
            Box {
                id: String,
                center: [f32; 3],
                half_size: [f32; 3],
            },
            Union {
                id: String,
                a: String,
                b: String,
            },
            Intersection {
                id: String,
                a: String,
                b: String,
            },
            Difference {
                id: String,
                a: String,
                b: String,
            },
        }
        let (id, shape) = match Wire::deserialize(deserializer)? {
            Wire::Sphere { id, center, radius } => (id, Shape::Sphere { center, radius }),
            Wire::Box {
                id,
                center,
                half_size,
            } => (id, Shape::Box { center, half_size }),
            Wire::Union { id, a, b } => (id, Shape::Union { a, b }),
            Wire::Intersection { id, a, b } => (id, Shape::Intersection { a, b }),
            Wire::Difference { id, a, b } => (id, Shape::Difference { a, b }),
        };
        Ok(Self { id, shape })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Sphere {
        center: [f32; 3],
        radius: f32,
    },
    Box {
        center: [f32; 3],
        half_size: [f32; 3],
    },
    Union {
        a: String,
        b: String,
    },
    Intersection {
        a: String,
        b: String,
    },
    Difference {
        a: String,
        b: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Instruction {
    Sphere([f32; 3], f32),
    Box([f32; 3], [f32; 3]),
    Union(usize, usize),
    Intersection(usize, usize),
    Difference(usize, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub(crate) code: Vec<Instruction>,
    pub(crate) root: usize,
}

pub fn point_valid(p: [f32; 3]) -> bool {
    p.iter().all(|x| x.is_finite() && x.abs() <= LIMIT)
}
fn size_valid(x: f32) -> bool {
    x.is_finite() && (0.0001..=LIMIT).contains(&x)
}
fn id_valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}

impl Document {
    /// Deterministic topological compilation, independent of source node ordering.
    /// Unreachable nodes are retained and validated, not silently discarded.
    pub fn compile(&self) -> Result<Model> {
        if self.version != 1 {
            return Err("unsupported document version; expected 1".into());
        }
        if self.nodes.is_empty() || self.nodes.len() > MAX_NODES {
            return Err(format!("node count must be 1..={MAX_NODES}"));
        }
        let mut nodes = BTreeMap::new();
        for node in &self.nodes {
            if !id_valid(&node.id) {
                return Err(format!("invalid node id: {}", node.id));
            }
            if nodes.insert(node.id.as_str(), &node.shape).is_some() {
                return Err(format!("duplicate node id: {}", node.id));
            }
        }
        if !nodes.contains_key(self.root.as_str()) {
            return Err("root does not exist".into());
        }
        let mut incoming = BTreeMap::new();
        let mut users: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut ready = BTreeSet::new();
        for (&id, shape) in &nodes {
            let deps = match shape {
                Shape::Sphere { center, radius } => {
                    if !point_valid(*center) || !size_valid(*radius) {
                        return Err(format!("invalid sphere: {id}"));
                    }
                    vec![]
                }
                Shape::Box { center, half_size } => {
                    if !point_valid(*center) || !half_size.iter().all(|v| size_valid(*v)) {
                        return Err(format!("invalid box: {id}"));
                    }
                    vec![]
                }
                Shape::Union { a, b }
                | Shape::Intersection { a, b }
                | Shape::Difference { a, b } => vec![a.as_str(), b.as_str()],
            };
            incoming.insert(id, deps.len());
            if deps.is_empty() {
                ready.insert(id);
            }
            for dep in deps {
                if !nodes.contains_key(dep) {
                    return Err(format!("missing dependency {dep} in {id}"));
                }
                users.entry(dep).or_default().push(id);
            }
        }
        let mut slots = BTreeMap::new();
        let mut code = Vec::with_capacity(nodes.len());
        while let Some(id) = ready.pop_first() {
            let op = match nodes[id] {
                Shape::Sphere { center, radius } => Instruction::Sphere(*center, *radius),
                Shape::Box { center, half_size } => Instruction::Box(*center, *half_size),
                Shape::Union { a, b } => Instruction::Union(slots[a.as_str()], slots[b.as_str()]),
                Shape::Intersection { a, b } => {
                    Instruction::Intersection(slots[a.as_str()], slots[b.as_str()])
                }
                Shape::Difference { a, b } => {
                    Instruction::Difference(slots[a.as_str()], slots[b.as_str()])
                }
            };
            slots.insert(id, code.len());
            code.push(op);
            if let Some(children) = users.get(id) {
                for child in children {
                    let count = incoming.get_mut(child).expect("known dependency");
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(child);
                    }
                }
            }
        }
        if code.len() != nodes.len() {
            return Err("cycle in modeling graph".into());
        }
        Ok(Model {
            root: slots[self.root.as_str()],
            code,
        })
    }
}

impl Model {
    pub fn instruction_count(&self) -> usize {
        self.code.len()
    }
    pub fn root_index(&self) -> usize {
        self.root
    }
    /// Negative inside, positive outside. Boolean combinations are conservative
    /// distance bounds; they need not be exact Euclidean signed distances.
    pub fn sample(&self, point: [f32; 3]) -> Result<f32> {
        if !point_valid(point) {
            return Err("sample point must be finite and within +/-1e6 meters".into());
        }
        Ok(self.eval(point, &mut Vec::new()))
    }
    pub(crate) fn eval(&self, p: [f32; 3], scratch: &mut Vec<f32>) -> f32 {
        scratch.clear();
        for op in &self.code {
            let value = match *op {
                Instruction::Sphere(c, r) => length(sub(p, c)) - r,
                Instruction::Box(c, h) => {
                    let q = std::array::from_fn::<_, 3, _>(|i| (p[i] - c[i]).abs() - h[i]);
                    length(q.map(|x| x.max(0.0))) + q[0].max(q[1]).max(q[2]).min(0.0)
                }
                Instruction::Union(a, b) => scratch[a].min(scratch[b]),
                Instruction::Intersection(a, b) => scratch[a].max(scratch[b]),
                Instruction::Difference(a, b) => scratch[a].max(-scratch[b]),
            };
            scratch.push(value);
        }
        scratch[self.root]
    }
}

pub(crate) fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn length(v: [f32; 3]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

pub fn parse_document(bytes: &[u8]) -> Result<Document> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err("document exceeds 2 MiB".into());
    }
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}
