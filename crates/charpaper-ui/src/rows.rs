use bevy::prelude::*;
use bevy::ui::UiSystems;

use crate::elements::Change;
use crate::elements::CyclerValue;
use crate::elements::FillBar;
use crate::elements::SegmentBar;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::widgets::*;

// Draws each row from what its tab says it shows. After Update, where the tabs work that out,
// and before Bevy lays the UI out, so a change shows on the same frame.
pub(crate) struct RowsPlugin;

impl Plugin for RowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click)
            .add_systems(PostUpdate, (draw_rows, draw_segments).before(UiSystems::Prepare));
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

// Made again whenever the choices or the current one change. They are a handful of buttons,
// and making them anew is simpler than working out which to relabel or restyle.
fn draw_segments(
    mut commands: Commands,
    rows: Query<(&UiContainer, &Shown), Changed<Shown>>,
    bars: Query<(Entity, &SegmentBar)>,
) {
    for (container, shown) in &rows {
        let (UiContainer::Row(cycler), Shown::Choice { labels, current }) = (container, shown)
        else {
            continue;
        };
        let Some((bar, _)) = bars.iter().find(|(_, bar)| bar.0 == *cycler) else {
            continue;
        };
        commands.entity(bar).despawn_children().with_children(|bar| {
            for (index, label) in labels.iter().enumerate() {
                bar.spawn(option_button(label, *cycler, index, *current == Some(index)));
            }
        });
    }
}

fn on_click(event: On<Pointer<Click>>, mut commands: Commands, buttons: Query<&UiButton>) {
    if let Ok(&UiButton::Choose(cycler, index)) = buttons.get(event.entity) {
        commands.trigger(Step { cycler, change: Change::To(index), continuous: false });
    }
}
