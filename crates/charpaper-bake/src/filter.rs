use std::f32::consts::PI;
use std::f32::consts::TAU;

use crate::cube;
use crate::cube::CubeLevel;
use crate::panorama::Panorama;
use crate::panorama::Rgb;

// Level i of n is what perceptual roughness i / (n - 1) sees; that's how Bevy picks one.
pub(crate) fn specular(source: &Panorama, size: u32, samples: u32) -> Vec<CubeLevel> {
    let count = size.max(1).ilog2() + 1;
    (0..count)
        .map(|level| {
            let face = (size >> level).max(1);
            if level == 0 {
                return cube::render(face, |dir| source.sharp(dir, face));
            }
            let roughness = level as f32 / (count - 1) as f32;
            cube::render(face, |dir| prefilter(source, dir, roughness * roughness, samples))
        })
        .collect()
}

// GGX prefiltering with the viewer along the normal (Karis 2013). Each sample reads the
// panorama blurred to its share of the lobe (filtered importance sampling, Křivánek &
// Colbert 2008), which keeps a small bright sun from speckling.
fn prefilter(source: &Panorama, normal: [f32; 3], alpha: f32, samples: u32) -> Rgb {
    let (tangent, bitangent) = basis(normal);
    let alpha2 = alpha * alpha;
    let texel = source.texel_solid_angle();
    let mut sum = [0.0; 3];
    let mut weight = 0.0;

    for i in 0..samples {
        let (xi1, xi2) = hammersley(i, samples);
        let phi = TAU * xi1;
        let cos_theta = ((1.0 - xi2) / (1.0 + (alpha2 - 1.0) * xi2)).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let half = [0, 1, 2].map(|k| {
            tangent[k] * sin_theta * phi.cos()
                + bitangent[k] * sin_theta * phi.sin()
                + normal[k] * cos_theta
        });
        let n_dot_h = cos_theta;
        let light = [0, 1, 2].map(|k| 2.0 * n_dot_h * half[k] - normal[k]);
        let n_dot_l = dot(normal, light);
        if n_dot_l <= 0.0 {
            continue;
        }

        // With the view along the normal, the pdf of `light` is D(h) / 4.
        let d = alpha2 / (PI * (n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0).powi(2));
        let covered = 1.0 / (samples as f32 * d / 4.0);
        let lod = 0.5 * (covered / texel).log2() + 1.0;

        let c = source.sample(light, lod);
        for k in 0..3 {
            sum[k] += c[k] * n_dot_l;
        }
        weight += n_dot_l;
    }
    sum.map(|s| if weight > 0.0 { s / weight } else { 0.0 })
}

// Summed directly over a 128×64 copy rather than through spherical harmonics, which can't
// hold a sun and ring around it. Divided by π so a white surface shows the light as is.
pub(crate) fn diffuse(source: &Panorama, size: u32) -> Vec<CubeLevel> {
    let texels: Vec<_> = source.texels(64).collect();
    vec![cube::render(size.max(1), |normal| {
        let mut irradiance = [0.0; 3];
        for &(dir, solid_angle, color) in &texels {
            let cosine = dot(normal, dir);
            if cosine > 0.0 {
                for k in 0..3 {
                    irradiance[k] += color[k] * cosine * solid_angle;
                }
            }
        }
        irradiance.map(|e| e / PI)
    })]
}

fn hammersley(i: u32, n: u32) -> (f32, f32) {
    (i as f32 / n as f32, i.reverse_bits() as f32 / 4_294_967_296.0)
}

fn basis(n: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let up = if n[1].abs() < 0.999 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let tangent = normalize(cross(up, n));
    (tangent, cross(n, tangent))
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    v.map(|c| c / length)
}
