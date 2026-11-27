use std::fs;
use std::ops::ControlFlow;
use std::path::Path;
use std::path::PathBuf;

use charpaper_pack::Method;
use charpaper_pack::Pack;
use charpaper_pack::PackError;
use charpaper_pack::PackWriter;
use charpaper_pack::check_path;
use charpaper_pack::pack_path;

fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("charpaper-pack-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

// Bytes zstd can't shrink, from a small xorshift so the tests need no crate for it.
fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn text(len: usize) -> Vec<u8> {
    b"{\"frames\": [0.0, 0.5, 1.0], \"shape_keys\": {}}\n"
        .iter()
        .copied()
        .cycle()
        .take(len)
        .collect()
}

fn write(writer: PackWriter, out: &Path) -> charpaper_pack::Summary {
    writer.write(out, |_| ControlFlow::Continue(())).unwrap()
}

fn suite(dir: &Path) -> PathBuf {
    let mut writer = PackWriter::new();
    writer.add_bytes("skins/casual.glb", noise(4000, 1)).unwrap();
    writer.add_bytes("skins/formal.glb", noise(3000, 2)).unwrap();
    writer.add_bytes("clips/idle.json", text(20_000)).unwrap();
    writer.add_bytes("environment/room/music/01 - Rain.ogg", text(5000)).unwrap();
    writer.add_bytes("environment/room.glb", noise(100, 3)).unwrap();
    writer.add_bytes("model.glb", noise(500, 4)).unwrap();
    let out = dir.join("suite.chr");
    write(writer, &out);
    out
}

#[test]
fn files_come_back_as_they_went_in() {
    let dir = folder("round-trip");
    let pack = Pack::open(suite(&dir)).unwrap();
    assert_eq!(pack.read("skins/casual.glb").unwrap(), noise(4000, 1));
    assert_eq!(pack.read("clips/idle.json").unwrap(), text(20_000));
    assert_eq!(pack.read("model.glb").unwrap(), noise(500, 4));
    pack.verify().unwrap();

    let paths: Vec<&str> = pack.entries().iter().map(|e| e.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);
    assert!(pack.entries().iter().all(|e| e.offset % 16 == 0));
}

#[test]
fn only_what_shrinks_is_compressed() {
    let dir = folder("methods");
    let pack = Pack::open(suite(&dir)).unwrap();
    let method = |path: &str| pack.entry(path).unwrap().method;
    assert_eq!(method("clips/idle.json"), Method::Zstd);
    assert_eq!(method("skins/casual.glb"), Method::Stored);
    // Text, but an .ogg is never tried.
    assert_eq!(method("environment/room/music/01 - Rain.ogg"), Method::Stored);
    let json = pack.entry("clips/idle.json").unwrap();
    assert!(json.stored < json.size / 10);
}

#[test]
fn folders_list_like_a_directory() {
    let dir = folder("listing");
    let pack = Pack::open(suite(&dir)).unwrap();
    let names = |dir: &str| pack.files_in(dir).map(|e| e.path.clone()).collect::<Vec<_>>();
    assert_eq!(names(""), ["model.glb"]);
    assert_eq!(names("skins"), ["skins/casual.glb", "skins/formal.glb"]);
    assert_eq!(names("environment"), ["environment/room.glb"]);
    assert_eq!(pack.folders_in(""), ["clips", "environment", "skins"]);
    assert_eq!(pack.folders_in("environment"), ["room"]);
    assert_eq!(pack.folders_in("environment/room"), ["music"]);
    assert!(pack.is_dir("environment/room/music"));
    assert!(pack.is_dir(""));
    assert!(!pack.is_dir("skin"));
    assert!(!pack.is_dir("skins/casual.glb"));
    assert!(names("nowhere").is_empty());
}

#[test]
fn the_same_files_make_the_same_pack() {
    let (one, two) = (folder("same-1"), folder("same-2"));
    assert_eq!(fs::read(suite(&one)).unwrap(), fs::read(suite(&two)).unwrap());
}

#[test]
fn identical_files_share_their_data() {
    let dir = folder("shared");
    let mut writer = PackWriter::new();
    writer.add_bytes("a.glb", noise(1000, 7)).unwrap();
    writer.add_bytes("b.glb", noise(1000, 7)).unwrap();
    let out = dir.join("suite.chr");
    let summary = write(writer, &out);
    assert_eq!(summary.shared, 1);
    let pack = Pack::open(&out).unwrap();
    assert_eq!(pack.entry("a.glb").unwrap().offset, pack.entry("b.glb").unwrap().offset);
    assert_eq!(pack.read("b.glb").unwrap(), noise(1000, 7));
}

#[test]
fn unchanged_files_are_copied_from_the_previous_pack() {
    let dir = folder("reuse");
    let out = suite(&dir);
    let mut writer = PackWriter::new();
    writer.reuse(Pack::open(&out).unwrap());
    writer.add_bytes("clips/idle.json", text(20_000)).unwrap();
    writer.add_bytes("skins/casual.glb", noise(4000, 99)).unwrap();
    let summary = write(writer, &out);
    assert_eq!(summary.files, 2);
    assert_eq!(summary.reused, 1);
    let pack = Pack::open(&out).unwrap();
    assert_eq!(pack.read("skins/casual.glb").unwrap(), noise(4000, 99));
    assert_eq!(pack.read("clips/idle.json").unwrap(), text(20_000));
    assert!(!pack.is_file("model.glb"));
}

#[test]
fn kept_files_carry_over_unless_added_again() {
    let dir = folder("keep");
    let out = suite(&dir);
    let mut writer = PackWriter::new();
    writer.reuse(Pack::open(&out).unwrap());
    writer.add_bytes("skins/casual.glb", noise(10, 5)).unwrap();
    writer.keep(|path| !path.starts_with("skins/"));
    writer.add_bytes("model.glb", noise(20, 6)).unwrap();
    write(writer, &out);

    let pack = Pack::open(&out).unwrap();
    assert_eq!(pack.read("skins/casual.glb").unwrap(), noise(10, 5));
    assert!(!pack.is_file("skins/formal.glb"));
    assert_eq!(pack.read("model.glb").unwrap(), noise(20, 6));
    assert_eq!(pack.read("clips/idle.json").unwrap(), text(20_000));
    pack.verify().unwrap();
}

#[test]
fn a_folder_packs_with_its_sub_folders() {
    let dir = folder("from-folder");
    let staged = dir.join("staged");
    fs::create_dir_all(staged.join("skins")).unwrap();
    fs::write(staged.join("skins").join("casual.glb"), noise(64, 8)).unwrap();
    fs::write(staged.join("model.glb"), noise(64, 9)).unwrap();
    fs::write(staged.join("suite.toml"), b"schema = 2").unwrap();
    let mut writer = PackWriter::new();
    writer.add_folder(&staged, |path| path != "suite.toml").unwrap();
    let out = dir.join("suite.chr");
    write(writer, &out);
    let pack = Pack::open(&out).unwrap();
    let paths: Vec<&str> = pack.entries().iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["model.glb", "skins/casual.glb"]);
    assert!(!dir.join("suite.chr.part").exists());
}

#[test]
fn bad_paths_are_refused() {
    for bad in ["", "/model.glb", "skins/", "skins//a.glb", "../a.glb", "a/./b", "a\\b", "C:a"] {
        assert!(check_path(bad).is_err(), "{bad:?} passed");
    }
    check_path("environment/room/music/01 - Rain.ogg").unwrap();
    assert_eq!(pack_path(Path::new("skins").join("a.glb").as_path()).unwrap(), "skins/a.glb");
    assert!(pack_path(Path::new("../a.glb")).is_err());
    let mut writer = PackWriter::new();
    assert!(writer.add_bytes("a//b", vec![]).is_err());
    writer.add_bytes("a.glb", vec![]).unwrap();
    assert!(matches!(writer.add_bytes("a.glb", vec![]), Err(PackError::Duplicate(_))));
}

#[test]
fn names_differing_only_in_capitals_are_refused() {
    let dir = folder("case");
    let mut writer = PackWriter::new();
    writer.add_bytes("skins/Casual.glb", vec![1]).unwrap();
    writer.add_bytes("skins/casual.glb", vec![2]).unwrap();
    let out = dir.join("suite.chr");
    assert!(matches!(
        writer.write(&out, |_| ControlFlow::Continue(())),
        Err(PackError::CaseClash(..))
    ));
    assert!(!out.exists());
}

#[test]
fn a_replaced_pack_is_noticed() {
    let dir = folder("replaced");
    let out = suite(&dir);
    let old = Pack::open(&out).unwrap();
    let mut writer = PackWriter::new();
    writer.add_bytes("model.glb", noise(500, 42)).unwrap();
    write(writer, &out);
    assert!(matches!(old.read("model.glb"), Err(PackError::Replaced(_))));
    assert_eq!(Pack::open(&out).unwrap().read("model.glb").unwrap(), noise(500, 42));
}

#[test]
fn damage_is_reported_not_read() {
    let dir = folder("damage");
    let out = suite(&dir);
    let good = fs::read(&out).unwrap();
    let damaged = |bytes: &[u8]| {
        fs::write(&out, bytes).unwrap();
        Pack::open(&out)
    };

    assert!(matches!(damaged(b"PK\x03\x04 not ours"), Err(PackError::NotAPack(_))));
    assert!(matches!(damaged(&good[..20]), Err(PackError::NotAPack(_))));
    assert!(matches!(damaged(&good[..good.len() - 1]), Err(PackError::Damaged { .. })));

    let mut newer = good.clone();
    newer[8] = 2;
    assert!(matches!(damaged(&newer), Err(PackError::Version { found: 2, .. })));

    let mut index = good.clone();
    *index.last_mut().unwrap() ^= 1;
    assert!(matches!(damaged(&index), Err(PackError::Damaged { .. })));

    // A stored file's byte: only verify can tell.
    let pack = damaged(&good).unwrap();
    let casual = pack.entry("skins/casual.glb").unwrap().offset as usize;
    let mut data = good.clone();
    data[casual] ^= 1;
    let pack = damaged(&data).unwrap();
    assert!(matches!(pack.verify(), Err(PackError::Damaged { .. })));

    // A compressed file's byte: zstd's own checksum catches it on the read.
    let json = pack.entry("clips/idle.json").unwrap();
    let mut data = good.clone();
    data[json.offset as usize + json.stored as usize / 2] ^= 0x55;
    let pack = damaged(&data).unwrap();
    assert!(matches!(pack.read("clips/idle.json"), Err(PackError::Damaged { .. })));
}

#[test]
fn a_missing_file_says_so() {
    let dir = folder("missing");
    let pack = Pack::open(suite(&dir)).unwrap();
    assert!(matches!(pack.read("skins/nope.glb"), Err(PackError::Missing { .. })));
}

#[test]
fn stopping_leaves_the_old_pack_as_it_was() {
    let dir = folder("stopped");
    let out = suite(&dir);
    let before = fs::read(&out).unwrap();
    let mut writer = PackWriter::new();
    writer.add_bytes("a.glb", noise(100, 1)).unwrap();
    writer.add_bytes("b.glb", noise(100, 2)).unwrap();
    let stop = |p: charpaper_pack::Progress| {
        if p.done == 1 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    };
    assert!(matches!(writer.write(&out, stop), Err(PackError::Stopped)));
    assert_eq!(fs::read(&out).unwrap(), before);
    assert!(!dir.join("suite.chr.part").exists());
}
