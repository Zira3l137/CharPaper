// A release's notes as cliff.toml writes them: `### Section` headings, each followed by one
// `- ` item per commit. An item holds the commit's whole message, body included, and ends
// with a link to the commit: `([1a2b3c4](https://github.com/…/commit/…))`. Only an item's
// first line is kept, which is the commit's subject.

use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notes {
    pub version: Version,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    // As cliff.toml names it, such as "Features" or "Other".
    pub title: String,
    pub items: Vec<String>,
}

pub(crate) fn parse(body: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    // Inside an item until the line ending with its commit link, so a commit body's own lines,
    // even one starting with `- ` or `#`, are never taken for items or headings. Notes with
    // no commit links, written by hand, have one line per item.
    let linked = body.contains("/commit/");
    let mut in_item = false;
    for line in body.lines() {
        let line = line.trim_end();
        if in_item {
            in_item = !ends_item(line);
            continue;
        }
        if let Some(title) = line.strip_prefix("### ") {
            sections.push(Section { title: title.trim().to_string(), items: Vec::new() });
        } else if let Some(item) = line.strip_prefix("- ") {
            in_item = linked && !ends_item(line);
            let text = plain(strip_link(item));
            match sections.last_mut() {
                Some(section) if !text.is_empty() => section.items.push(text),
                _ => {}
            }
        }
    }
    sections.retain(|section| !section.items.is_empty());
    sections
}

fn ends_item(line: &str) -> bool {
    line.ends_with("))") && line.contains("/commit/")
}

fn strip_link(item: &str) -> &str {
    match item.rfind(" ([") {
        Some(at) if ends_item(item) => &item[..at],
        _ => item,
    }
}

// Markdown's bold and code marks, which the panel would show as they are.
fn plain(text: &str) -> String {
    text.replace("**", "").replace('`', "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cut from 0.6.0's notes.
    const NOTES: &str = "\
## 0.6.0 - 2026-11-18

### Fixes

- **release:** Update cargo.lock ([3ae8f08](https://github.com/Zira3l137/CharPaper/commit/3ae8f08))

### Other

- Carry lights' Radius and keyed Strength and Color

glTF has no place for either.
- not an item, but a line of the body
### nor a heading
follow_character now shares. ([5857a01](https://github.com/Zira3l137/CharPaper/commit/5857a01))
- Read glTF light `colors` as linear ([ca1b0f4](https://github.com/Zira3l137/CharPaper/commit/ca1b0f4))
";

    #[test]
    fn keeps_each_commits_subject() {
        let sections = parse(NOTES);
        assert_eq!(
            sections,
            vec![
                Section { title: "Fixes".into(), items: vec!["release: Update cargo.lock".into()] },
                Section {
                    title: "Other".into(),
                    items: vec![
                        "Carry lights' Radius and keyed Strength and Color".into(),
                        "Read glTF light colors as linear".into(),
                    ],
                },
            ]
        );
    }

    #[test]
    fn notes_written_by_hand_still_read() {
        assert_eq!(parse("Just a sentence."), vec![]);
        let sections = parse("### Fixes\n- A fix\n- Another\n");
        assert_eq!(sections[0].items, vec!["A fix".to_string(), "Another".to_string()]);
    }
}
