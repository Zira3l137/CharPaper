use std::fs;
use std::path::Path;

use crate::cube::parse_cube;
use crate::inspect::Report;
use crate::inspect::Severity;
use crate::layout::Suite;
use crate::manifest::MANIFEST_FILE;

// A LUT that fails to load leaves the image ungraded, so problems with the one in use are
// errors and the rest warnings.
pub(super) fn check_luts(suite: &Suite, report: &mut Report) {
    let chosen = suite.post.lut.name();
    for lut in &suite.luts {
        let severity =
            if chosen == Some(lut.name.as_str()) { Severity::Error } else { Severity::Warning };
        let problem = fs::read_to_string(suite.absolute(&lut.file))
            .map_err(|e| e.to_string())
            .and_then(|text| parse_cube(&text).map(drop));
        if let Err(message) = problem {
            report.push(severity, Some(&lut.file), message);
        }
    }

    if let Some(name) = chosen
        && !suite.luts.iter().any(|l| l.name == name)
    {
        let message =
            format!("`post.lut` names {name:?}, which is not in `luts/`, so no LUT is applied");
        report.push(Severity::Warning, Some(Path::new(MANIFEST_FILE)), message);
    }
}
