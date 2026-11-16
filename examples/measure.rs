//! Reproducible storage experiment, NOT a compressed-mesh or GPU benchmark.
use serde_json::json;
use std::{hint::black_box, time::Instant};
use t3dnyere::{Document, Model, Node, Shape};

fn sphere_mesh(lat: u32, lon: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut positions = vec![[0.0, 1.0, 0.0]];
    for ring in 1..lat {
        let phi = std::f32::consts::PI * ring as f32 / lat as f32;
        for j in 0..lon {
            let theta = std::f32::consts::TAU * j as f32 / lon as f32;
            positions.push([phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()]);
        }
    }
    let bottom = positions.len() as u32;
    positions.push([0.0, -1.0, 0.0]);
    let mut indices = vec![];
    for j in 0..lon {
        let next = (j + 1) % lon;
        indices.extend([0, 1 + next, 1 + j]);
        for ring in 0..lat - 2 {
            let a = 1 + ring * lon + j;
            let b = 1 + ring * lon + next;
            indices.extend([a, b, a + lon, b, b + lon, a + lon]);
        }
        let last = 1 + (lat - 2) * lon;
        indices.extend([bottom, last + j, last + next]);
    }
    (positions, indices)
}
fn main() {
    let doc = Document {
        version: 1,
        root: "sphere".into(),
        nodes: vec![Node {
            id: "sphere".into(),
            shape: Shape::Sphere {
                center: [0.0; 3],
                radius: 1.0,
            },
        }],
    };
    let model = doc.compile().unwrap();
    let bytes = model.to_bytes();
    let mut baselines = vec![];
    for (lat, lon) in [(8, 16), (32, 64)] {
        let (positions, indices) = sphere_mesh(lat, lon);
        let mut raw = vec![];
        for p in &positions {
            for v in p {
                raw.extend(v.to_le_bytes());
            }
        }
        for index in &indices {
            raw.extend(index.to_le_bytes());
        }
        baselines.push(json!({"latitudes":lat,"longitudes":lon,"vertices":positions.len(),"triangles":indices.len()/3,"position_f32_index_u32_bytes":raw.len()}));
    }
    let start = Instant::now();
    for _ in 0..10000 {
        black_box(Model::from_bytes(black_box(&bytes)).unwrap());
    }
    let decode_us = start.elapsed().as_micros();
    let start = Instant::now();
    for i in 0..100000 {
        black_box(
            model
                .sample(black_box([(i % 100) as f32 * 0.03, 0.2, 0.3]))
                .unwrap(),
        );
    }
    let sample_us = start.elapsed().as_micros();
    println!(
        "{}",
        json!({
            "experiment":"one unit sphere, no normals/UV/materials; raw mesh baselines are not compressed; different surface discretization",
            "t3d_bytes":bytes.len(),"authoring_compact_json_bytes":serde_json::to_vec(&doc).unwrap().len(),
            "mesh_baselines":baselines,"decode_iterations":10000,"decode_total_us":decode_us,
            "sample_iterations":100000,"sample_total_us":sample_us,
            "timing_note":"single local run, includes allocation, not statistically controlled; not a GPU or end-to-end engine benchmark"
        })
    );
}
