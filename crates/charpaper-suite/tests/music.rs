use std::path::Path;

use charpaper_suite::track_title;

#[test]
fn the_order_prefix_is_left_out() {
    let title = |name: &str| track_title(Path::new("environment/room/music").join(name).as_path());
    assert_eq!(title("01 - Rain on glass.ogg"), "Rain on glass");
    assert_eq!(title("112 - Last.ogg"), "Last");
    assert_eq!(title("1979.ogg"), "1979");
    assert_eq!(title("99 Luftballons.ogg"), "99 Luftballons");
    assert_eq!(title("01 - .ogg"), "01 - ");
    assert_eq!(title("Intro - 01.ogg"), "Intro - 01");
}
