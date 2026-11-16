//! T3D v1: explicit little-endian wire encoding, no struct layout/transmute.
use crate::{Instruction, MAX_NODES, Model, Result, point_valid};

pub const MAGIC: &[u8; 8] = b"T3DNYERE";
pub const HEADER_BYTES: usize = 20;
pub const RECORD_BYTES: usize = 32;

impl Model {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_BYTES + RECORD_BYTES * self.code.len());
        bytes.extend(MAGIC);
        for word in [1u32, self.code.len() as u32, self.root as u32] {
            bytes.extend(word.to_le_bytes());
        }
        for op in &self.code {
            let mut words = [0u32; 8];
            match *op {
                Instruction::Sphere(c, r) => {
                    words[0] = 1;
                    for i in 0..3 {
                        words[i + 1] = c[i].to_bits();
                    }
                    words[4] = r.to_bits();
                }
                Instruction::Box(c, h) => {
                    words[0] = 2;
                    for i in 0..3 {
                        words[i + 1] = c[i].to_bits();
                        words[i + 4] = h[i].to_bits();
                    }
                }
                Instruction::Union(a, b)
                | Instruction::Intersection(a, b)
                | Instruction::Difference(a, b) => {
                    words[0] = match op {
                        Instruction::Union(..) => 3,
                        Instruction::Intersection(..) => 4,
                        _ => 5,
                    };
                    words[1] = a as u32;
                    words[2] = b as u32;
                }
            }
            for word in words {
                bytes.extend(word.to_le_bytes());
            }
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_BYTES || &bytes[..8] != MAGIC {
            return Err("invalid T3D magic or truncated header".into());
        }
        let word = |offset| {
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("bounded word"))
        };
        if word(8) != 1 {
            return Err("unsupported T3D version".into());
        }
        let count = word(12) as usize;
        let root = word(16) as usize;
        if count == 0 || count > MAX_NODES || root >= count {
            return Err("invalid T3D instruction count or root".into());
        }
        if bytes.len() != HEADER_BYTES + count * RECORD_BYTES {
            return Err("T3D length mismatch (truncated or trailing data)".into());
        }
        let mut code = Vec::with_capacity(count);
        for i in 0..count {
            let words: [u32; 8] =
                std::array::from_fn(|j| word(HEADER_BYTES + i * RECORD_BYTES + j * 4));
            let vec3 = |start| std::array::from_fn(|j| f32::from_bits(words[start + j]));
            let reserved = |start: usize| words[start..].iter().all(|x| *x == 0);
            let op = match words[0] {
                1 if reserved(5) => {
                    let c = vec3(1);
                    let r = f32::from_bits(words[4]);
                    if !point_valid(c) || !super::size_valid(r) {
                        return Err("invalid T3D sphere values".into());
                    }
                    Instruction::Sphere(c, r)
                }
                2 if reserved(7) => {
                    let c = vec3(1);
                    let h: [f32; 3] = vec3(4);
                    if !point_valid(c) || !h.iter().all(|v| super::size_valid(*v)) {
                        return Err("invalid T3D box values".into());
                    }
                    Instruction::Box(c, h)
                }
                tag @ 3..=5 if reserved(3) => {
                    let a = words[1] as usize;
                    let b = words[2] as usize;
                    if a >= i || b >= i {
                        return Err("T3D dependencies must reference earlier instructions".into());
                    }
                    match tag {
                        3 => Instruction::Union(a, b),
                        4 => Instruction::Intersection(a, b),
                        _ => Instruction::Difference(a, b),
                    }
                }
                _ => return Err("unknown T3D opcode or nonzero reserved fields".into()),
            };
            code.push(op);
        }
        Ok(Self { code, root })
    }
}
