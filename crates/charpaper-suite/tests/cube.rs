use charpaper_suite::parse_cube;

// Every entry its own input, so a lookup returns what went in.
fn identity(size: usize, header: &str) -> String {
    let mut text = format!("{header}\nLUT_3D_SIZE {size}\n");
    let last = (size - 1) as f32;
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                text += &format!("{} {} {}\n", r as f32 / last, g as f32 / last, b as f32 / last);
            }
        }
    }
    text
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
}

#[test]
fn a_3d_table_reads_with_red_changing_fastest() {
    let lut = parse_cube(&identity(3, "TITLE \"identity\"\n# made by hand")).unwrap();
    assert_eq!(lut.size, 3);
    assert_eq!(lut.at(2, 0, 0), [1.0, 0.0, 0.0]);
    assert_eq!(lut.at(0, 0, 2), [0.0, 0.0, 1.0]);
    assert!(close(lut.sample([0.25, 0.6, 0.9]), [0.25, 0.6, 0.9]));
}

#[test]
fn a_1d_table_becomes_three_curves_on_a_3d_grid() {
    // Red inverted, green halved, blue untouched.
    let text = "LUT_1D_SIZE 2\n1 0 0\n0 0.5 1\n";
    let lut = parse_cube(text).unwrap();
    assert_eq!(lut.size, 65);
    assert!(close(lut.sample([0.25, 0.5, 0.75]), [0.75, 0.25, 0.75]));
}

#[test]
fn a_domain_other_than_0_to_1_is_resampled_onto_it() {
    // An identity over 0..2: an input of 0.5 reads the entry for 0.5, which is 0.25.
    let lut = parse_cube(&identity(3, "DOMAIN_MIN 0 0 0\nDOMAIN_MAX 2 2 2")).unwrap();
    assert!(close(lut.sample([0.5, 1.0, 0.0]), [0.25, 0.5, 0.0]));
    let lut = parse_cube(&identity(3, "LUT_3D_INPUT_RANGE 0 2")).unwrap();
    assert!(close(lut.sample([0.5, 1.0, 0.0]), [0.25, 0.5, 0.0]));
}

#[test]
fn broken_tables_say_what_is_wrong() {
    let cases = [
        ("0 0 0\n", "no LUT_3D_SIZE or LUT_1D_SIZE"),
        ("LUT_3D_SIZE 2\n0 0 0\n", "has 1 entries where its size needs 8"),
        ("LUT_3D_SIZE 300\n", "outside 2 to 129"),
        ("LUT_3D_SIZE 2\n0 0\n", "line 2: an entry needs exactly three numbers"),
        ("LUT_3D_SIZE 2\nLUT_1D_SIZE 2\n", "both a 1D and a 3D table"),
        ("LUT_3D_SIZE x\n", "line 1: LUT_3D_SIZE needs a whole number"),
    ];
    for (text, expected) in cases {
        let err = parse_cube(text).unwrap_err();
        assert!(err.contains(expected), "{text:?}: {err}");
    }
    let flipped = identity(2, "DOMAIN_MIN 1 1 1\nDOMAIN_MAX 0 0 0");
    assert!(parse_cube(&flipped).unwrap_err().contains("DOMAIN_MAX must be above"));
}
