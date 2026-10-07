use bevy::prelude::*;
use bevy::ui::UiSystems;

use crate::elements::CyclerValue;
use crate::elements::FillBar;
use crate::elements::Shown;
use crate::elements::UiContainer;
use crate::widgets::*;

// Draws each row from what its tab says it shows. After Update, where the tabs work that out,
// and before Bevy lays the UI out, so a change shows on the same frame.
pub(crate) struct RowsPlugin;

impl Plugin for RowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, draw_rows.before(UiSystems::Prepare));
    }
}

fn draw_rows(
    rows: Query<(&UiContainer, &Shown), Changed<Shown>>,
    mut values: Query<(&CyclerValue, &mut Text)>,
    mut bars: Query<(&FillBar, &mut Node)>,
) {
    for (container, shown) in &rows {
        let UiContainer::Row(cycler) = *container else {
            continue;
        };
        let (text, fill, centered) = match shown {
            Shown::Nothing => continue,
            Shown::Number { text, fill, centered } => (text.as_str(), *fill, *centered),
            Shown::Choice { labels, current } => {
                let label = current.and_then(|current| labels.get(current));
                (label.map_or("-", String::as_str), 0.0, false)
            }
        };
        set_value(&mut values, cycler, text);
        let (left, width) =
            if centered { (fill.min(0.5), (fill - 0.5).abs()) } else { (0.0, fill) };
        for (bar, mut node) in &mut bars {
            if bar.0 == cycler {
                node.left = Val::Percent(left * 100.0);
                node.width = Val::Percent(width * 100.0);
            }
        }
    }
}
