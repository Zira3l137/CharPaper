use bevy::prelude::*;
use bevy::ui::UiSystems;

use crate::elements::Change;
use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::FillBar;
use crate::elements::PickerList;
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
        app.init_resource::<OpenPicker>().add_observer(on_click).add_systems(
            PostUpdate,
            (draw_rows, draw_choices, show_open_list.run_if(resource_changed::<OpenPicker>))
                .before(UiSystems::Prepare),
        );
    }
}

// The picker whose list is open. One at a time: opening another closes it.
#[derive(Resource, Default)]
struct OpenPicker(Option<Cycler>);

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

// Segment buttons and list items are made again whenever the choices or the current one change.
// They are a handful of buttons, and making them anew is simpler than working out which to
// relabel or restyle.
fn draw_choices(
    mut commands: Commands,
    rows: Query<(&UiContainer, &Shown), Changed<Shown>>,
    bars: Query<(Entity, &SegmentBar)>,
    lists: Query<(Entity, &PickerList)>,
) {
    for (container, shown) in &rows {
        let (&UiContainer::Row(cycler), Shown::Choice { labels, current }) = (container, shown)
        else {
            continue;
        };
        let current = *current;
        if let Some((bar, _)) = bars.iter().find(|(_, bar)| bar.0 == cycler) {
            commands.entity(bar).despawn_children().with_children(|bar| {
                for (index, label) in labels.iter().enumerate() {
                    bar.spawn(option_button(label, cycler, index, current == Some(index)));
                }
            });
        }
        if let Some((list, _)) = lists.iter().find(|(_, list)| list.0 == cycler) {
            commands.entity(list).despawn_children().with_children(|list| {
                for (index, label) in labels.iter().enumerate() {
                    list.spawn(list_item(label, cycler, index, current == Some(index)));
                }
            });
        }
    }
}

fn show_open_list(open: Res<OpenPicker>, mut lists: Query<(&PickerList, &mut Node)>) {
    for (list, mut node) in &mut lists {
        node.display = display(open.0 == Some(list.0));
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&UiButton>,
    mut open: ResMut<OpenPicker>,
) {
    match buttons.get(event.entity) {
        Ok(&UiButton::Choose(cycler, index)) => {
            commands.trigger(Step { cycler, change: Change::To(index), continuous: false });
            if open.0 == Some(cycler) {
                open.0 = None;
            }
        }
        Ok(&UiButton::Open(cycler)) => {
            open.0 = if open.0 == Some(cycler) { None } else { Some(cycler) };
        }
        _ => {}
    }
}
