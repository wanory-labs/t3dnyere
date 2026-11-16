//! Deliberately small CPU preview, not a production renderer.
use crate::{Model, Result, length, point_valid, sub};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub vertical_fov_degrees: f32,
    pub max_distance: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: [4.0, 3.0, 5.0],
            target: [0.0; 3],
            vertical_fov_degrees: 45.0,
            max_distance: 100.0,
        }
    }
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = length(v);
    v.map(|x| x / l)
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn preview(model: &Model, width: u32, height: u32, camera: Camera) -> Result<Vec<u8>> {
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return Err("preview dimensions must be 1..=512".into());
    }
    // Bound total work: 256 steps plus 6 gradient evaluations, <=100M node evals.
    if u64::from(width) * u64::from(height) * 262 * model.instruction_count() as u64 > 100_000_000 {
        return Err("preview work budget exceeded; reduce dimensions or node count".into());
    }
    if !point_valid(camera.eye)
        || !point_valid(camera.target)
        || !camera.vertical_fov_degrees.is_finite()
        || !(1.0..=150.0).contains(&camera.vertical_fov_degrees)
        || !camera.max_distance.is_finite()
        || !(0.01..=10000.0).contains(&camera.max_distance)
        || length(sub(camera.target, camera.eye)) < 0.0001
    {
        return Err("invalid preview camera".into());
    }
    let forward = normalize(sub(camera.target, camera.eye));
    let up_hint = if forward[1].abs() > 0.99 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = normalize(cross(forward, up_hint));
    let up = cross(right, forward);
    let spread = (camera.vertical_fov_degrees.to_radians() * 0.5).tan();
    let mut output = format!("P6\n{width} {height}\n255\n").into_bytes();
    let mut scratch = Vec::with_capacity(model.instruction_count());
    let light = normalize([0.5, 0.8, 0.6]);
    for y in 0..height {
        for x in 0..width {
            let sx = (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * width as f32 / height as f32
                * spread;
            let sy = (1.0 - 2.0 * (y as f32 + 0.5) / height as f32) * spread;
            let ray = normalize(std::array::from_fn(|i| {
                forward[i] + right[i] * sx + up[i] * sy
            }));
            let mut t = 0.0;
            let mut color = [19, 25, 34];
            for _ in 0..256 {
                let p = std::array::from_fn(|i| camera.eye[i] + ray[i] * t);
                let d = model.eval(p, &mut scratch);
                if d.abs() < 0.0005 {
                    let e = 0.001;
                    let gradient = std::array::from_fn(|i| {
                        let mut a = p;
                        let mut b = p;
                        a[i] += e;
                        b[i] -= e;
                        model.eval(a, &mut scratch) - model.eval(b, &mut scratch)
                    });
                    let lighting = if length(gradient) > 1e-8 {
                        0.18 + 0.82 * dot(normalize(gradient), light).max(0.0)
                    } else {
                        0.18
                    };
                    color = [102.0, 192.0, 214.0].map(|v| (v * lighting) as u8);
                    break;
                }
                t += d.abs().max(0.0001);
                if t > camera.max_distance {
                    break;
                }
            }
            output.extend(color);
        }
    }
    Ok(output)
}
