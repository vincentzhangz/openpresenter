//! The slide groups and the labels that go with them.
//!
//! A slide carries a group — "Verse 1", "Chorus", "Tag" — and the group is
//! what the operator navigates by during a show. The standard groups are
//! the ones a service is built from; the operator adds their own when the
//! service needs one the app does not know about.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, TestSupportExt, div, px};

use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// The groups a service is built from.
///
/// The names are the ones the app colours already, so a group always reads
/// in the same colour wherever it is shown.
pub const DEFAULT_GROUPS: [&str; 24] = [
    "Blank",
    "Verse",
    "Verse 1",
    "Verse 2",
    "Verse 3",
    "Verse 4",
    "Verse 5",
    "Verse 6",
    "Chorus",
    "Chorus 1",
    "Chorus 2",
    "Chorus 3",
    "Chorus 4",
    "Bridge",
    "Bridge 1",
    "Bridge 2",
    "Bridge 3",
    "PreChorus",
    "Tag",
    "Turnaround",
    "Vamp",
    "Intro",
    "Outro",
    "Ending",
];

/// The labels a slide can carry besides its group.
///
/// A label says what the slide is for: the same song twice, once as a
/// chorus and once as a repeated chorus.
pub const DEFAULT_LABELS: [&str; 4] = ["Blank Slide", "Instrumental", "Repeat", "Spoken Word"];

/// A group the operator added.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomGroup {
    /// The group's name.
    pub name: String,
    /// The group's colour, as 0xRRGGBB.
    pub color: u32,
}

/// A group the operator can pick.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupEntry {
    /// The group's name.
    pub name: String,
    /// The group's colour, as 0xRRGGBB.
    pub color: u32,
    /// Whether the operator added the group themselves.
    pub custom: bool,
}

/// The groups the operator has, and the standard set.
#[derive(Debug, Clone, Default)]
pub struct GroupsState {
    /// The groups the operator added, in the order they were added.
    pub custom: Vec<CustomGroup>,
    /// The name the operator is typing for a new group.
    pub new_name: String,
}

impl GroupsState {
    /// The state the app starts with.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every group the operator can pick: the standard set, then their own.
    pub fn all(&self) -> Vec<GroupEntry> {
        let mut groups: Vec<GroupEntry> = DEFAULT_GROUPS
            .iter()
            .map(|name| GroupEntry {
                name: (*name).to_string(),
                color: standard_group_color(name),
                custom: false,
            })
            .collect();
        for custom in &self.custom {
            groups.push(GroupEntry {
                name: custom.name.clone(),
                color: custom.color,
                custom: true,
            });
        }
        groups
    }

    /// Add a group of the operator's own.
    ///
    /// A name that is already taken is not added twice: two groups with one
    /// name would be two groups the operator cannot tell apart.
    pub fn add(&mut self, name: String) -> bool {
        let name = name.trim().to_string();
        if name.is_empty() {
            return false;
        }
        if self.all().iter().any(|group| group.name == name) {
            return false;
        }
        // The colour is picked from the palette the app already uses, so a
        // group the operator adds reads like one the app knows.
        let color = custom_group_color(&name);
        self.custom.push(CustomGroup { name, color });
        true
    }

    /// Take a group of the operator's own away.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.custom.len();
        self.custom.retain(|group| group.name != name);
        self.custom.len() != before
    }

    /// Give a group a colour.
    pub fn set_color(&mut self, name: &str, color: u32) -> bool {
        match self.custom.iter_mut().find(|group| group.name == name) {
            Some(group) => {
                group.color = color;
                true
            }
            None => false,
        }
    }
}

/// The colour of one of the standard groups.
///
/// The colour is the one the app already gives the group everywhere else,
/// so a group never reads in two colours at once.
pub fn standard_group_color(name: &str) -> u32 {
    // The colour is the one the app already gives the group everywhere
    // else, so a group never reads in two colours at once.
    crate::ui::components::group_option_rgb(name)
}

/// The colour a group the operator added is given.
///
/// The colour is picked from the palette the app uses, so the groups the
/// operator adds sit beside the standard ones rather than beside nothing.
fn custom_group_color(name: &str) -> u32 {
    let hash: usize = name.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    let (r, g, b) =
        crate::ui::components::GROUP_COLORS[hash % crate::ui::components::GROUP_COLORS.len()];
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Messages owned by the Groups feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Open the groups and labels window.
    Open,
    /// Close the groups and labels window.
    Close,
    /// The operator picked a group for the slide being edited.
    SetGroup(String),
    /// The operator typed a new group's name.
    NewGroupNameChanged(String),
    /// The operator added a group of their own.
    AddGroup,
    /// The operator took a group of their own away.
    RemoveGroup(String),
    /// The operator gave a group a colour.
    SetGroupColor(String, u32),
    /// The operator picked one of the standard labels.
    SetLabel(String),
}

/// Apply a message from the groups window.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Open => {
            w.shell.groups_open = true;
        }
        Message::Close => {
            w.shell.groups_open = false;
        }
        Message::SetGroup(group) => {
            set_slide_group(w, &group, cx);
        }
        Message::NewGroupNameChanged(name) => {
            w.groups.new_name = name;
        }
        Message::AddGroup => {
            let name = w.groups.new_name.clone();
            if w.groups.add(name) {
                // The name is cleared, so the operator can add the next one
                // without clearing the field first.
                w.groups.new_name.clear();
            }
        }
        Message::RemoveGroup(name) => {
            w.groups.remove(&name);
        }
        Message::SetGroupColor(name, color) => {
            w.groups.set_color(&name, color);
        }
        Message::SetLabel(label) => {
            set_slide_label(w, &label, cx);
        }
    }
    cx.notify();
}

/// Put the group on the slide being edited.
fn set_slide_group(w: &mut MainWindow, group: &str, cx: &mut Context<MainWindow>) {
    let group = group.trim().to_string();
    let group = if group.is_empty() { None } else { Some(group) };
    let Some(slide) = w.get_current_slide_mut() else {
        return;
    };
    if slide.group == group {
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.group = group;
    }
    crate::ui::layers::save_current_slide(w, cx);
    cx.notify();
}

/// Put the label on the slide being edited.
///
/// The label is kept in the slide's notes, under a line of its own, so it
/// travels with the slide and is not lost when the slide is saved.
fn set_slide_label(w: &mut MainWindow, label: &str, cx: &mut Context<MainWindow>) {
    let Some(slide) = w.get_current_slide_mut() else {
        return;
    };
    let label = label.trim();
    let mut notes = slide.notes.clone().unwrap_or_default();
    // The label is the first line, and it is marked so it can be told from
    // the operator's own notes.
    notes = notes
        .lines()
        .filter(|line| !line.starts_with(LABEL_PREFIX))
        .collect::<Vec<_>>()
        .join("\n");
    if !label.is_empty() {
        notes = format!("{LABEL_PREFIX}{label}\n{notes}");
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.notes = Some(notes.trim_start().to_string());
    }
    crate::ui::layers::save_current_slide(w, cx);
    cx.notify();
}

/// The mark that tells the operator's label from their notes.
const LABEL_PREFIX: &str = "Label: ";

/// The label a slide carries, if it has one.
pub fn slide_label(slide: &crate::domain::Slide) -> Option<String> {
    let notes = slide.notes.as_deref()?;
    notes
        .lines()
        .find_map(|line| line.strip_prefix(LABEL_PREFIX))
        .map(|label| label.trim().to_string())
}

/// The groups and labels window.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let current_group = w
        .get_current_slide()
        .and_then(|slide| slide.group.clone())
        .unwrap_or_default();
    let current_label = w
        .get_current_slide()
        .and_then(slide_label)
        .unwrap_or_default();
    let groups = w.groups.all();

    // How many slides each group is actually on: the count tells the
    // operator at a glance which groups their shows use.
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for pres in w.editor.presentations.iter().chain(w.editor.editing.iter()) {
        for slide in &pres.slides {
            if let Some(group) = &slide.group {
                *counts.entry(group.clone()).or_default() += 1;
            }
        }
    }

    let mut panel = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    // Header: title + count + the add row, so adding a group is one action
    // beside the title instead of a button under a wall of text.
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(format!("Groups · {}", groups.len())),
            )
            .child(div().flex_1())
            .child(
                div()
                    .w(px(180.))
                    .child(Input::new(&w.groups_name_input.clone())),
            )
            .child(
                Button::new("group-add")
                    .label("Add Group")
                    .secondary()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Groups(crate::ui::groups::Message::AddGroup),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    // All groups as border chips with the group's colour, its slide count,
    // and a remove control for the operator's own groups.
    let mut grid = div().flex().flex_wrap().gap(px(6.));
    for entry in &groups {
        let active = current_group == entry.name;
        let count = counts.get(&entry.name).copied().unwrap_or(0);
        grid = grid.child(group_chip_entry(entry, active, count, cx));
    }
    panel = panel.child(grid);

    // The labels.
    panel = panel.child(
        div()
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Labels"),
    );
    let mut labels = div().flex().flex_wrap().gap(px(6.));
    for label in DEFAULT_LABELS {
        let active = current_label == label;
        labels = labels.child(label_chip(label, active, cx));
    }
    panel = panel.child(labels);

    panel
}

/// One group as a border chip: colour badge, count of slides on the group,
/// and (for the operator's own groups) a remove control. Clicking an active
/// chip clears the group.
fn group_chip_entry(
    entry: &GroupEntry,
    active: bool,
    count: usize,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let name = entry.name.clone();
    let name_click = name.clone();
    let color = entry.color;
    div()
        .id(SharedString::from(format!("group-{}", entry.name)))
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .py(px(4.))
        .px(px(8.))
        .rounded(px(12.))
        .border_1()
        .border_color(if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            let next = if w
                .get_current_slide()
                .and_then(|slide| slide.group.as_deref())
                == Some(name_click.as_str())
            {
                String::new()
            } else {
                name_click.clone()
            };
            w.handle_message(
                RootMessage::Groups(crate::ui::groups::Message::SetGroup(next)),
                window,
                cx,
            );
        }))
        .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(theme::c(color)))
        .child(
            div()
                .text_size(px(11.))
                .text_color(if active {
                    theme::c(theme::TEXT_PRIMARY)
                } else {
                    theme::c(theme::TEXT_SECONDARY)
                })
                .child(name.clone()),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("×{count}")),
        )
        .when(entry.custom, |chip| {
            chip.child(
                Button::new(SharedString::from(format!("group-remove-{name}")))
                    .label("×")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener({
                        let name = name.clone();
                        move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::Groups(crate::ui::groups::Message::RemoveGroup(
                                    name.clone(),
                                )),
                                window,
                                cx,
                            );
                        }
                    })),
            )
        })
}

/// One slide label as a quiet border chip.
fn label_chip(label: &'static str, active: bool, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("group-label-{label}")))
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .py(px(4.))
        .px(px(8.))
        .rounded(px(12.))
        .border_1()
        .border_color(if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .text_size(px(11.))
        .text_color(if active {
            theme::c(theme::TEXT_PRIMARY)
        } else {
            theme::c(theme::TEXT_SECONDARY)
        })
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(
                RootMessage::Groups(crate::ui::groups::Message::SetLabel(String::from(label))),
                window,
                cx,
            );
        }))
        .child(label)
}

/// The picker of groups, for the slide's own group.
///
/// The picker is small: it sits on the slide's own panel, where the
/// operator is already choosing what the slide is.
pub fn group_picker(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let current = w
        .get_current_slide()
        .and_then(|slide| slide.group.clone())
        .unwrap_or_default();

    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .flex_wrap();

    // The groups that are in use on this slide come first: the operator is
    // choosing between the groups this show uses, not between every group
    // there is.
    let mut names: Vec<String> = Vec::new();
    if let Some(presentation) = w.editor.editing.as_ref() {
        for slide in &presentation.slides {
            if let Some(group) = &slide.group
                && !names.contains(group)
            {
                names.push(group.clone());
            }
        }
    }
    for name in DEFAULT_GROUPS {
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
    }

    for name in names {
        let active = current == name;
        let color = standard_group_color(&name);
        row = row.child(
            div()
                .id(SharedString::from(format!("group-pick-{name}")))
                .test_support()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(5.))
                .py(px(3.))
                .px(px(8.))
                .rounded(px(12.))
                .border_1()
                .border_color(if active {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::BORDER_PANEL)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .text_size(px(11.))
                .text_color(if active {
                    theme::c(theme::TEXT_PRIMARY)
                } else {
                    theme::c(theme::TEXT_SECONDARY)
                })
                .on_click(cx.listener({
                    let name = name.clone();
                    move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Groups(crate::ui::groups::Message::SetGroup(name.clone())),
                            window,
                            cx,
                        );
                    }
                }))
                .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(theme::c(color)))
                .child(name),
        );
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A group that is already there is not added twice.
    #[test]
    fn a_group_is_not_added_twice() {
        let mut state = GroupsState::new();
        assert!(state.add(String::from("Sung")));
        assert!(!state.add(String::from("Sung")));
        assert!(!state.add(String::from("Verse 1")));
        assert_eq!(state.custom.len(), 1);
    }

    /// The standard groups come first, then the operator's own.
    #[test]
    fn the_standard_groups_come_first() {
        let mut state = GroupsState::new();
        state.add(String::from("Sung"));
        let all = state.all();
        assert_eq!(all[0].name, "Blank");
        assert!(!all[0].custom);
        assert_eq!(all.last().map(|group| group.name.as_str()), Some("Sung"));
        assert!(all.last().expect("the group is there").custom);
    }

    /// A standard group reads in the colour the app already gives it.
    ///
    /// The group is shown in the slide list, in the picker and in the
    /// inspector, so the three must not disagree about its colour.
    #[test]
    fn a_standard_group_reads_in_the_apps_colour() {
        let state = GroupsState::new();
        let groups = state.all();
        let verse = groups
            .iter()
            .find(|group| group.name == "Verse 1")
            .expect("the verse is a standard group");
        assert_eq!(verse.color, standard_group_color("Verse 1"));
        // The verses are blue, so the operator tells them apart at a glance.
        let red = ((verse.color >> 16) & 0xFF) as u8;
        let blue = (verse.color & 0xFF) as u8;
        assert!(blue > red, "a verse reads blue");
    }
}
