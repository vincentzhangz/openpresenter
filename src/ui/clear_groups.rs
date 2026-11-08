//! The clear groups.
//!
//! The output is built from layers — the slide, the media, the props, the
//! messages, the audio and the live feed — and each of them can be cleared
//! on its own. A clear group is a named set of those layers, so one press
//! clears "everything but the slide" and another clears the slide and the
//! sound and leaves the props up.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// One layer of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The slide on the output.
    Presentation,
    /// The slide's own video.
    PresentationMedia,
    /// The lower thirds and the logos.
    Props,
    /// The operator's own note.
    Messages,
    /// What the app says to the room on its own.
    Announcements,
    /// The music.
    Music,
}

impl Layer {
    /// The name the layer is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Presentation => "Presentation",
            Self::PresentationMedia => "Presentation Media",
            Self::Props => "Props",
            Self::Messages => "Messages",
            Self::Announcements => "Announcements",
            Self::Music => "Music",
        }
    }

    /// Every layer, in the order the output is built.
    pub const ALL: [Layer; 6] = [
        Layer::Presentation,
        Layer::PresentationMedia,
        Layer::Props,
        Layer::Messages,
        Layer::Announcements,
        Layer::Music,
    ];
}

/// A named set of layers that one press clears.
#[derive(Debug, Clone, PartialEq)]
pub struct ClearGroup {
    /// The group's name.
    pub name: String,
    /// The layers the group clears.
    pub layers: Vec<Layer>,
}

/// The clear groups the operator has.
#[derive(Debug, Clone, Default)]
pub struct ClearGroupsState {
    /// The groups, in the order they are shown.
    pub groups: Vec<ClearGroup>,
    /// The group the operator is working on.
    pub selected: usize,
    /// The name the operator is typing for a new group.
    pub new_name: String,
}

impl ClearGroupsState {
    /// The groups the operator starts with.
    pub fn default_groups() -> Vec<ClearGroup> {
        vec![
            ClearGroup {
                name: String::from("All"),
                layers: Layer::ALL.to_vec(),
            },
            ClearGroup {
                name: String::from("Slides only"),
                layers: vec![Layer::Presentation],
            },
            ClearGroup {
                name: String::from("Everything but the slide"),
                layers: vec![
                    Layer::PresentationMedia,
                    Layer::Props,
                    Layer::Messages,
                    Layer::Announcements,
                    Layer::Music,
                ],
            },
            ClearGroup {
                name: String::from("Picture and sound"),
                layers: vec![Layer::Presentation, Layer::PresentationMedia, Layer::Music],
            },
        ]
    }

    /// The state the app starts with.
    pub fn new() -> Self {
        Self {
            groups: Self::default_groups(),
            selected: 0,
            new_name: String::new(),
        }
    }

    /// The group the operator is working on.
    pub fn selected(&self) -> Option<&ClearGroup> {
        self.groups.get(self.selected)
    }

    /// Pick the group the operator is working on.
    pub fn select(&mut self, index: usize) {
        if index < self.groups.len() {
            self.selected = index;
        }
    }

    /// Add a group, with the layers of the group that was selected.
    ///
    /// A new group starts as a copy of the one the operator was working on,
    /// because that is the group they are most likely to want to change.
    pub fn add(&mut self, name: String) -> bool {
        let name = name.trim().to_string();
        if name.is_empty() {
            return false;
        }
        if self.groups.iter().any(|group| group.name == name) {
            return false;
        }
        let layers = self
            .selected()
            .map(|group| group.layers.clone())
            .unwrap_or_default();
        self.groups.push(ClearGroup { name, layers });
        self.selected = self.groups.len() - 1;
        true
    }

    /// Take a group away.
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.groups.len() {
            return false;
        }
        self.groups.remove(index);
        if self.selected >= self.groups.len() {
            self.selected = self.groups.len().saturating_sub(1);
        }
        true
    }

    /// Rename a group.
    pub fn rename(&mut self, index: usize, name: String) -> bool {
        let name = name.trim().to_string();
        if name.is_empty() {
            return false;
        }
        match self.groups.get_mut(index) {
            Some(group) => {
                group.name = name;
                true
            }
            None => false,
        }
    }

    /// Turn a layer in a group on or off.
    pub fn toggle_layer(&mut self, index: usize, layer: Layer) -> bool {
        let Some(group) = self.groups.get_mut(index) else {
            return false;
        };
        if let Some(position) = group.layers.iter().position(|item| *item == layer) {
            group.layers.remove(position);
        } else {
            group.layers.push(layer);
        }
        true
    }
}

/// Messages owned by the Clear Groups feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Open the clear groups window.
    Open,
    /// Close the clear groups window.
    Close,
    /// Clear the layers of one group.
    Clear(usize),
    /// The operator picked the group they are working on.
    Select(usize),
    /// The operator typed a name for a new group.
    NewNameChanged(String),
    /// The operator added a group.
    Add,
    /// The operator took the group they are working on away.
    Remove,
    /// The operator turned a layer in the group on or off.
    ToggleLayer(Layer),
}

/// Apply a message from the clear groups window.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Open => {
            w.shell.clear_groups_open = true;
        }
        Message::Close => {
            w.shell.clear_groups_open = false;
        }
        Message::Clear(index) => {
            let layers = w
                .clear_groups
                .groups
                .get(index)
                .map(|group| group.layers.clone())
                .unwrap_or_default();
            crate::ui::navigation::clear_layers(w, &layers, cx);
        }
        Message::Select(index) => {
            w.clear_groups.select(index);
        }
        Message::NewNameChanged(name) => {
            w.clear_groups.new_name = name;
        }
        Message::Add => {
            let name = w.clear_groups.new_name.clone();
            if w.clear_groups.add(name) {
                w.clear_groups.new_name.clear();
            }
        }
        Message::Remove => {
            let index = w.clear_groups.selected;
            w.clear_groups.remove(index);
        }
        Message::ToggleLayer(layer) => {
            let index = w.clear_groups.selected;
            w.clear_groups.toggle_layer(index, layer);
        }
    }
    cx.notify();
}

/// The clear groups window.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let groups = w.clear_groups.groups.clone();
    let selected = w.clear_groups.selected;
    let new_name = w.clear_groups.new_name.clone();

    let mut panel = div()
        .w(px(400.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    // The groups.
    panel = panel.child(
        div()
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Clear Groups"),
    );
    let mut list = div().flex().flex_col().gap(px(4.));
    for (index, group) in groups.iter().enumerate() {
        let active = index == selected;
        list = list.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    Button::new(SharedString::from(format!("clear-group-{index}")))
                        .label(group.name.clone())
                        .ghost()
                        .xsmall()
                        .when(active, |button| button.primary())
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::ClearGroups(crate::ui::clear_groups::Message::Select(
                                    index,
                                )),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("clear-group-do-{index}")))
                        .label("Clear")
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::ClearGroups(crate::ui::clear_groups::Message::Clear(
                                    index,
                                )),
                                window,
                                cx,
                            );
                        })),
                ),
        );
    }
    panel = panel.child(list);

    // The layers of the group the operator is working on.
    if let Some(group) = groups.get(selected) {
        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("Layers in “{}”", group.name)),
        );
        let mut layers = div().flex().flex_wrap().gap(px(4.));
        for layer in Layer::ALL {
            let in_group = group.layers.contains(&layer);
            layers = layers.child(
                Button::new(SharedString::from(format!("clear-layer-{}", layer.label())))
                    .label(layer.label())
                    .ghost()
                    .xsmall()
                    .when(in_group, |button| button.primary())
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::ClearGroups(
                                crate::ui::clear_groups::Message::ToggleLayer(layer),
                            ),
                            window,
                            cx,
                        );
                    })),
            );
        }
        panel = panel.child(layers);
    }

    // A new group.
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .w(px(200.))
                    .child(Input::new(&w.clear_groups_name_input.clone())),
            )
            .child(
                Button::new("clear-group-add")
                    .label("Add Group")
                    .secondary()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            RootMessage::ClearGroups(crate::ui::clear_groups::Message::Add),
                            window,
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("clear-group-remove")
                    .label("Remove")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            RootMessage::ClearGroups(crate::ui::clear_groups::Message::Remove),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    let _ = new_name;
    panel
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The groups the operator starts with are the ones a show needs.
    #[test]
    fn the_groups_the_operator_starts_with() {
        let groups = ClearGroupsState::default_groups();
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0].layers.len(), Layer::ALL.len());
        assert_eq!(groups[1].layers, vec![Layer::Presentation]);
    }

    /// A layer goes in and out of a group.
    #[test]
    fn a_layer_goes_in_and_out_of_a_group() {
        let mut state = ClearGroupsState::new();
        // The second group holds the slide alone, so the props go in and
        // come out of it.
        let index = 1;
        assert!(!state.groups[index].layers.contains(&Layer::Props));
        assert!(state.toggle_layer(index, Layer::Props));
        assert!(state.groups[index].layers.contains(&Layer::Props));
        assert!(state.toggle_layer(index, Layer::Props));
        assert!(!state.groups[index].layers.contains(&Layer::Props));
    }

    /// A group that is already there is not added twice.
    #[test]
    fn a_group_is_not_added_twice() {
        let mut state = ClearGroupsState::new();
        assert!(!state.add(String::from("All")));
        assert!(state.add(String::from("Sung")));
        assert_eq!(state.groups.len(), 5);
    }
}
