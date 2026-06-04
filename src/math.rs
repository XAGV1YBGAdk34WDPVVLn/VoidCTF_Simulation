// src/math.rs
// Basic 3D vector helper functions.

pub fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn length_sq(a: [f32; 3]) -> f32 {
    a[0] * a[0] + a[1] * a[1] + a[2] * a[2]
}

pub fn length(a: [f32; 3]) -> f32 {
    length_sq(a).sqrt()
}

pub fn normalize(a: [f32; 3]) -> [f32; 3] {
    let len = length(a);
    if len > 0.00001 {
        scale(a, 1.0 / len)
    } else {
        [0.0, 0.0, 0.0]
    }
}

pub fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    length(sub(a, b))
}
