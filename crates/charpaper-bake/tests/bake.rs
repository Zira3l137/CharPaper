use std::fs;
use std::path::PathBuf;

use charpaper_bake::BakeSettings;
use charpaper_bake::Map;
use charpaper_bake::bake;
use charpaper_bake::missing;
use image::Rgb32FImage;

const SETTINGS: BakeSettings =
    BakeSettings { skybox_size: 16, specular_size: 8, specular_samples: 16, diffuse_size: 4 };

/// A bright sky over dark ground, with a small very bright sun low in the sky
/// towards Blender's +X, which is the image's centre column.
fn panorama(folder: &PathBuf) -> PathBuf {
    let (width, height) = (128, 64);
    let image = Rgb32FImage::from_fn(width, height, |x, y| {
        let sun = (x as i32 - 64).abs() <= 1 && (y as i32 - 26).abs() <= 1;
        let color = if sun {
            [500.0, 500.0, 500.0]
        } else if y < height / 2 {
            [2.0, 2.0, 2.0]
        } else {
            [0.1, 0.05, 0.0]
        };
        image::Rgb(color)
    });
    let path = folder.join("sky.hdr");
    image.save(&path).unwrap();
    path
}

fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("charpaper-bake-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

struct Ktx2 {
    size: u32,
    faces: u32,
    levels: Vec<Vec<[f32; 3]>>,
}

fn read(path: &PathBuf) -> Ktx2 {
    let bytes = fs::read(path).unwrap();
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let long = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
    assert_eq!(&bytes[1..7], b"KTX 20");
    assert_eq!(word(12), 123, "vkFormat should be E5B9G9R9_UFLOAT_PACK32");
    let levels = (0..word(40) as usize)
        .map(|i| {
            let (offset, length) = (long(80 + 24 * i), long(88 + 24 * i));
            bytes[offset..offset + length]
                .chunks(4)
                .map(|c| unpack(u32::from_le_bytes(c.try_into().unwrap())))
                .collect()
        })
        .collect();
    Ktx2 { size: word(20), faces: word(36), levels }
}

fn unpack(packed: u32) -> [f32; 3] {
    let scale = 2f32.powi((packed >> 27) as i32 - 24);
    [packed & 511, (packed >> 9) & 511, (packed >> 18) & 511].map(|m| m as f32 * scale)
}

fn face(map: &Ktx2, level: usize, face: usize) -> &[[f32; 3]] {
    let texels = map.levels[level].len() / 6;
    &map.levels[level][face * texels..(face + 1) * texels]
}

fn mean(texels: &[[f32; 3]]) -> f32 {
    texels.iter().map(|t| t[0]).sum::<f32>() / texels.len() as f32
}

#[test]
fn bakes_the_three_maps_the_right_way_up() {
    let folder = folder("maps");
    let source = panorama(&folder);
    let written = bake(&source, &folder, &SETTINGS).unwrap();
    assert_eq!(written.len(), 3);

    let skybox = read(&folder.join("skybox.ktx2"));
    assert_eq!((skybox.size, skybox.faces, skybox.levels.len()), (16, 6, 1));
    // Faces are +X, -X, +Y, -Y, +Z, -Z.
    assert!(mean(face(&skybox, 0, 2)) > 1.5, "up should be sky");
    assert!(mean(face(&skybox, 0, 3)) < 0.2, "down should be ground");
    let brightest = (0..6).max_by(|&a, &b| {
        let peak = |f| face(&skybox, 0, f).iter().map(|t| t[0]).fold(0.0, f32::max);
        peak(a).total_cmp(&peak(b))
    });
    assert_eq!(brightest, Some(0), "the sun at Blender +X should land on the +X face");

    let specular = read(&folder.join("specular.ktx2"));
    assert_eq!((specular.size, specular.levels.len()), (8, 4));
    assert_eq!(specular.levels[3].len(), 6, "the last level is 1×1 per face");

    let diffuse = read(&folder.join("diffuse.ktx2"));
    assert_eq!((diffuse.size, diffuse.levels.len()), (4, 1));
    // Face averages include texels tilted up to 45° off the axis, so down
    // still catches some sky: the gap is clear, not total.
    let [px, _, up, down, ..] = [0, 1, 2, 3, 4, 5].map(|f| mean(face(&diffuse, 0, f)));
    assert!(up > down * 3.0, "up {up}, down {down}");
    assert!(px > up, "+X {px}, up {up}");
}

#[test]
fn only_missing_maps_are_baked() {
    let folder = folder("missing");
    let source = panorama(&folder);
    bake(&source, &folder, &SETTINGS).unwrap();
    let before = fs::read(folder.join("specular.ktx2")).unwrap();

    assert!(missing(&folder).is_empty());
    assert!(bake(&source, &folder, &SETTINGS).unwrap().is_empty());

    fs::remove_file(folder.join("diffuse.ktx2")).unwrap();
    assert_eq!(missing(&folder), [Map::Diffuse]);
    let written = bake(&source, &folder, &SETTINGS).unwrap();
    assert_eq!(written, [folder.join("diffuse.ktx2")]);
    assert_eq!(fs::read(folder.join("specular.ktx2")).unwrap(), before);
}
