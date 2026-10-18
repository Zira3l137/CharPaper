mod arrows;
mod wheel;
mod widgets;

use bevy::ecs::component::Mutable;
use bevy::ecs::system::SystemParam;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use charpaper_scene::FocusedStage;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use crate::locale::Locale;
use crate::locale::Tr;
use crate::panel::new_panel;
use crate::section::Content;
use crate::widgets::*;

use widgets::Choose;
use widgets::FillBar;
use widgets::Open;
use widgets::PickerList;
use widgets::SegmentBar;
use widgets::Value;
use widgets::list_item;
use widgets::option_button;

// A row of the panel is declared once, as a Setting, where the page lays it out. Its `show`
// says what it shows, worked out again whenever what the tabs read from changes, and its
// `change` is what a click, a held arrow or the wheel does to it. The widgets around it, the
// stepper, segments or picker, are the same for every row.
pub(crate) struct RowsPlugin;

impl Plugin for RowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OpenPicker>()
            .add_plugins((wheel::WheelPlugin, arrows::ArrowsPlugin))
            .add_observer(on_click)
            .add_systems(
                PostUpdate,
                (
                    show_rows.run_if(crate::tabs::sources_changed),
                    draw_rows,
                    show_open_list.run_if(resource_changed::<OpenPicker>.or_eager(new_panel)),
                )
                    .chain()
                    .in_set(DrawRows)
                    .before(UiSystems::Prepare),
            );
    }
}

// Every row on screen is up to date once this has run.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DrawRows;

// Plain functions rather than systems, so a row is a few lines: they take the whole world, and
// FocusedWorld reaches into the stage the panel shows.
pub(crate) type ShowFn = fn(&World, &FluentLanguageLoader) -> Option<Shown>;
pub(crate) type ChangeFn = fn(&mut World, &Step);

pub(crate) struct Setting {
    pub label: Tr,
    // The sentence at the bottom of the panel while the pointer is on the row.
    pub help: Tr,
    // None while there is nothing to show yet, before the stage has loaded, say: the row keeps
    // what it showed.
    pub show: ShowFn,
    pub change: ChangeFn,
}

impl Setting {
    fn row(self, accelerates: bool) -> Row {
        Row { help: self.help, show: self.show, change: self.change, accelerates }
    }
}

#[derive(Component)]
#[require(Shown, Hovered, Content)]
pub(crate) struct Row {
    pub help: Tr,
    show: ShowFn,
    change: ChangeFn,
    // For numbers: the faster the wheel spins, the further each notch goes. A list moves one
    // choice per notch so none is skipped unseen.
    pub accelerates: bool,
}

// What a row shows, from its `show`. The row's widgets draw it.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub(crate) enum Shown {
    // Not there at all: it would do nothing for what is on screen.
    #[default]
    Hidden,
    // A number as text, and where it sits in its range from 0 to 1. A centered range, one with
    // its neutral value in the middle, fills out from the middle.
    Number {
        text: String,
        fill: f32,
        centered: bool,
    },
    // A list of choices by their labels, and which one is current.
    Choice {
        labels: Vec<String>,
        current: Option<usize>,
    },
}

impl Shown {
    pub(crate) fn when(self, shown: bool) -> Self {
        if shown { self } else { Shown::Hidden }
    }
}

// Steps through a row's values.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Step {
    pub change: Change,
    // From the wheel or a held arrow rather than a single click.
    pub continuous: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Change {
    // Places to move, forward when positive.
    By(i32),
    // A choice picked straight from a list, by its place in it.
    To(usize),
}

impl Step {
    // A click goes round from the last value to the first; spinning the wheel stops at the end
    // instead of landing back where it started.
    pub(crate) fn wraps(&self) -> bool {
        !self.continuous
    }

    // How far a number moves. A number is never picked from a list, so a pick moves it nowhere.
    pub(crate) fn by(&self) -> i32 {
        match self.change {
            Change::By(by) => by,
            Change::To(_) => 0,
        }
    }

    // A command, which gets the whole world, as the row's `change` wants. Nothing happens if the
    // row is gone meanwhile, with its panel.
    pub(crate) fn on(self, row: Entity) -> impl Command {
        move |world: &mut World| {
            if let Some(change) = world.get::<Row>(row).map(|row| row.change) {
                change(world, &self);
            }
        }
    }
}

// What rows show and change: the components of the stage the panel shows, which is the one
// under the cursor. Resources are the world's own `resource` and `resource_mut`.
pub(crate) trait FocusedWorld {
    fn focused<T: Component>(&self) -> Option<&T>;
    fn focused_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>>;
    fn edit<T: Component<Mutability = Mutable>>(&mut self, edit: impl FnOnce(&mut T));
}

impl FocusedWorld for World {
    fn focused<T: Component>(&self) -> Option<&T> {
        self.get(self.get_resource::<FocusedStage>()?.0?)
    }

    fn focused_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
        let stage = self.get_resource::<FocusedStage>()?.0?;
        self.get_mut(stage)
    }

    fn edit<T: Component<Mutability = Mutable>>(&mut self, edit: impl FnOnce(&mut T)) {
        if let Some(mut value) = self.focused_mut::<T>() {
            edit(&mut *value);
        }
    }
}

// The row a part of it belongs to: the nearest one above it, or itself.
#[derive(SystemParam)]
pub(crate) struct RowOf<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    rows: Query<'w, 's, &'static Row>,
}

impl RowOf<'_, '_> {
    pub(crate) fn get(&self, part: Entity) -> Option<(Entity, &Row)> {
        std::iter::once(part)
            .chain(self.parents.iter_ancestors(part))
            .find_map(|entity| self.rows.get(entity).ok().map(|row| (entity, row)))
    }
}

pub(crate) fn number(text: impl Into<String>, value: f32, min: f32, max: f32) -> Shown {
    Shown::Number { text: text.into(), fill: fraction(value, min, max), centered: false }
}

// For a range with its neutral value in the middle, such as warmth.
pub(crate) fn balance(text: impl Into<String>, value: f32, min: f32, max: f32) -> Shown {
    Shown::Number { text: text.into(), fill: fraction(value, min, max), centered: true }
}

// max and min rather than clamp: they turn a NaN, say from the log of a zero, into 0.
fn fraction(value: f32, min: f32, max: f32) -> f32 {
    if max > min { ((value - min) / (max - min)).max(0.0).min(1.0) } else { 0.0 }
}

pub(crate) fn choice<T: PartialEq>(
    options: &[T],
    current: &T,
    label: impl Fn(&T) -> String,
) -> Shown {
    let labels = options.iter().map(label).collect();
    Shown::Choice { labels, current: options.iter().position(|option| option == current) }
}

pub(crate) fn switch(locale: &FluentLanguageLoader, on: bool) -> Shown {
    choice(&[false, true], &on, |&on| on_off(locale, on))
}

pub(crate) fn on_off(locale: &FluentLanguageLoader, on: bool) -> String {
    if on { fl!(locale, "value-on") } else { fl!(locale, "value-off") }
}

pub(crate) fn percent(fraction: f32) -> String {
    format!("{:.0}%", fraction * 100.0)
}

// With a sign, and none on zero, which also keeps a rounded -0 from showing.
pub(crate) fn signed_percent(fraction: f32) -> String {
    let percent = (fraction * 100.0).round();
    if percent == 0.0 { "0%".into() } else { format!("{percent:+.0}%") }
}

// The label of a choice that is a name, or none.
pub(crate) fn name(option: &Option<String>) -> String {
    option.clone().unwrap_or_else(|| "-".into())
}

// The option the step lands on. Moving by places goes round past either end when the step
// wraps and stops there otherwise. An unknown current value moves to the first option.
pub(crate) fn stepped<T: PartialEq + Clone>(options: &[T], current: &T, step: &Step) -> Option<T> {
    let len = options.len() as i32;
    let index = match (step.change, options.iter().position(|o| o == current)) {
        (Change::To(index), _) => index as i32,
        (Change::By(by), Some(i)) if step.wraps() => (i as i32 + by).rem_euclid(len),
        (Change::By(by), Some(i)) => (i as i32 + by).clamp(0, len - 1),
        (Change::By(_), None) => 0,
    };
    options.get(index as usize).cloned()
}

pub(crate) fn pick<T: PartialEq + Clone>(options: &[T], value: &mut T, step: &Step) {
    if let Some(next) = stepped(options, value, step) {
        *value = next;
    }
}

// A switch is a list of Off and On: a click flips it, going up turns it on, going down off.
pub(crate) fn flip(on: &mut bool, step: &Step) {
    pick(&[false, true], on, step);
}

// How a number moves: `step` a place, between `min` and `max`, rounded to `1 / per_unit` so
// that repeated steps don't drift into values like 0.15000001.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scale {
    pub step: f32,
    pub min: f32,
    pub max: f32,
    pub per_unit: f32,
}

impl Scale {
    pub(crate) fn nudge(self, value: f32, step: &Step) -> f32 {
        let next = (value + step.by() as f32 * self.step).clamp(self.min, self.max);
        (next * self.per_unit).round() / self.per_unit
    }

    pub(crate) fn number(self, text: impl Into<String>, value: f32) -> Shown {
        number(text, value, self.min, self.max)
    }

    pub(crate) fn balance(self, text: impl Into<String>, value: f32) -> Shown {
        balance(text, value, self.min, self.max)
    }
}

// The picker whose list is open. One at a time: opening another closes it.
#[derive(Resource, Default)]
struct OpenPicker(Option<Entity>);

// Every row works its value out again, but a row's Shown is only touched when that changed, so
// only those rows are drawn again.
fn show_rows(world: &mut World, rows: &mut QueryState<(Entity, &'static Row)>) {
    let shown: Vec<(Entity, Shown)> = {
        let world: &World = world;
        let locale: &FluentLanguageLoader = world.resource::<Locale>();
        let show = |(entity, row): (Entity, &Row)| Some((entity, (row.show)(world, locale)?));
        rows.iter(world).filter_map(show).collect()
    };
    for (entity, shown) in shown {
        if let Some(mut current) = world.get_mut::<Shown>(entity) {
            current.set_if_neq(shown);
        }
    }
}

// Segment buttons and list items are made again whenever the choices or the current one change.
// They are a handful of buttons, and making them anew is simpler than working out which to
// relabel or restyle. Each row fills only the parts inside it: every panel has its own.
fn draw_rows(
    mut commands: Commands,
    mut rows: Query<(Entity, &Shown, &mut Node), (With<Row>, Changed<Shown>)>,
    children: Query<&Children>,
    mut values: Query<&mut Text, With<Value>>,
    mut bars: Query<&mut Node, (With<FillBar>, Without<Row>)>,
    segments: Query<(), With<SegmentBar>>,
    lists: Query<(), With<PickerList>>,
) {
    for (row, shown, mut node) in &mut rows {
        let visible = display(*shown != Shown::Hidden);
        if node.display != visible {
            node.display = visible;
        }
        let (text, fill, centered) = match shown {
            Shown::Hidden => continue,
            Shown::Number { text, fill, centered } => (text.as_str(), *fill, *centered),
            Shown::Choice { labels, current } => {
                let label = current.and_then(|current| labels.get(current));
                (label.map_or("-", String::as_str), 0.0, false)
            }
        };
        let (left, width) =
            if centered { (fill.min(0.5), (fill - 0.5).abs()) } else { (0.0, fill) };
        for part in children.iter_descendants(row) {
            if let Ok(mut value) = values.get_mut(part) {
                value.0 = text.to_string();
            }
            if let Ok(mut bar) = bars.get_mut(part) {
                bar.left = Val::Percent(left * 100.0);
                bar.width = Val::Percent(width * 100.0);
            }
            let Shown::Choice { labels, current } = shown else {
                continue;
            };
            let current = *current;
            if segments.contains(part) {
                commands.entity(part).despawn_children().with_children(|bar| {
                    for (index, label) in labels.iter().enumerate() {
                        bar.spawn(option_button(label, index, current == Some(index)));
                    }
                });
            } else if lists.contains(part) {
                commands.entity(part).despawn_children().with_children(|list| {
                    for (index, label) in labels.iter().enumerate() {
                        list.spawn(list_item(label, index, current == Some(index)));
                    }
                });
            }
        }
    }
}

fn show_open_list(
    open: Res<OpenPicker>,
    row_of: RowOf,
    mut lists: Query<(Entity, &mut Node), With<PickerList>>,
) {
    for (list, mut node) in &mut lists {
        let row = row_of.get(list).map(|(row, _)| row);
        node.display = display(open.0.is_some() && open.0 == row);
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<(Option<&Choose>, Has<Open>)>,
    row_of: RowOf,
    mut open: ResMut<OpenPicker>,
) {
    let (choose, opens) = match buttons.get(event.entity) {
        Ok((None, false)) | Err(_) => return,
        Ok(found) => found,
    };
    let Some((row, _)) = row_of.get(event.entity) else {
        return;
    };
    if let Some(&Choose(index)) = choose {
        commands.queue(Step { change: Change::To(index), continuous: false }.on(row));
        if open.0 == Some(row) {
            open.0 = None;
        }
    } else if opens {
        open.0 = if open.0 == Some(row) { None } else { Some(row) };
    }
}
