//! The test pattern window.
//!
//! A test pattern tells the operator that the projector is alive and that
//! the picture is the picture they think it is. It belongs to the screen,
//! not to the slide: the pattern is on the output whatever the operator has
//! queued, and it comes off when the operator says so.

use gpui_kit::component::Disableable;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::render::test_pattern::Pattern;
use crate::ui::main_window::MainWindow;
use crate::ui::theme;

/// Where the pattern is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Location {
    /// On every screen.
    #[default]
    AllScreens,
    /// On the audience screens only.
    AudienceScreens,
    /// On the stage screens only.
    StageScreens,
}

impl Location {
    /// The name the location is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::AllScreens => "All Screens",
            Self::AudienceScreens => "All Audience Screens",
            Self::StageScreens => "All Stage Screens",
        }
    }

    /// Every location the operator can pick.
    pub const ALL: [Location; 3] = [
        Location::AllScreens,
        Location::AudienceScreens,
        Location::StageScreens,
    ];
}

/// The test pattern the app is showing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TestPatternState {
    /// The pattern on the screen, if any.
    pub pattern: Option<Pattern>,
    /// Where the pattern is shown.
    pub location: Location,
    /// The level of the black and the white ramps.
    pub black_level: u8,
    pub white_level: u8,
}

impl TestPatternState {
    /// The pattern the given screen shows.
    ///
    /// A pattern is on the stage screens as well as the audience screens
    /// when the operator asks for every screen: the stage is the operator's
    /// own screen, and a projector that is not working is the first thing
    /// the operator needs to know.
    pub fn pattern_for(&self, screen_id: &str) -> Option<Pattern> {
        let pattern = self.pattern?;
        let is_stage = crate::ui::compositor::is_stage_screen(screen_id);
        match self.location {
            Location::AllScreens => Some(pattern),
            Location::AudienceScreens => {
                if is_stage {
                    None
                } else {
                    Some(pattern)
                }
            }
            Location::StageScreens => {
                if is_stage {
                    Some(pattern)
                } else {
                    None
                }
            }
        }
    }
}

/// Messages owned by the Test Pattern feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Open the test pattern window.
    Open,
    /// Close the test pattern window.
    Close,
    /// The operator picked a pattern.
    SetPattern(Option<Pattern>),
    /// The operator picked where the pattern is shown.
    SetLocation(Location),
    /// The operator changed the level of the black ramp.
    BlackLevelChanged(u8),
    /// The operator changed the level of the white ramp.
    WhiteLevelChanged(u8),
    /// The operator turned the pattern on or off.
    ToggleShow,
}

/// The pattern a screen shows, or nothing.
pub(crate) fn pattern_for_screen(w: &MainWindow, screen_id: &str) -> Option<Pattern> {
    w.test_pattern.pattern_for(screen_id)
}

/// Apply a message from the test pattern window.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Open => {
            w.shell.test_pattern_open = true;
        }
        Message::Close => {
            w.shell.test_pattern_open = false;
        }
        Message::SetPattern(pattern) => {
            w.test_pattern.pattern = pattern;
            w.shell.test_pattern_open = true;
            // The screen has to be redrawn: the pattern is over the picture,
            // so the picture the operator sees is not the picture that was
            // there a moment ago.
            crate::ui::compositor::invalidate_all(w);
        }
        Message::SetLocation(location) => {
            w.test_pattern.location = location;
            crate::ui::compositor::invalidate_all(w);
        }
        Message::BlackLevelChanged(level) => {
            w.test_pattern.black_level = level;
            if matches!(w.test_pattern.pattern, Some(Pattern::BlackLevel(_))) {
                w.test_pattern.pattern = Some(Pattern::BlackLevel(level));
                crate::ui::compositor::invalidate_all(w);
            }
        }
        Message::WhiteLevelChanged(level) => {
            w.test_pattern.white_level = level;
            if matches!(w.test_pattern.pattern, Some(Pattern::WhiteLevel(_))) {
                w.test_pattern.pattern = Some(Pattern::WhiteLevel(level));
                crate::ui::compositor::invalidate_all(w);
            }
        }
        Message::ToggleShow => {
            w.test_pattern.pattern = match w.test_pattern.pattern {
                Some(_) => None,
                None => Some(Pattern::ColorBars),
            };
            crate::ui::compositor::invalidate_all(w);
        }
    }
    cx.notify();
}

/// The test pattern window.
///
/// The window is a panel on the right of the operator's screen: the pattern
/// is on the projector, and the operator has to be able to see what they
/// turned on.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let state = w.test_pattern;
    let showing = state.pattern.is_some();

    let mut panel = div()
        .w(px(420.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    // The pattern.
    panel = panel.child(
        div()
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Test Pattern"),
    );
    let mut grid = div().flex().flex_wrap().gap(px(4.));
    for pattern in Pattern::ALL {
        let active = state.pattern == Some(pattern);
        grid = grid.child(
            Button::new(SharedString::from(format!(
                "test-pattern-{}",
                pattern.label().replace(' ', "-")
            )))
            .label(pattern.label())
            .ghost()
            .xsmall()
            .when(active, |button| button.primary())
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    crate::ui::messages::Message::TestPattern(Message::SetPattern(Some(pattern))),
                    window,
                    cx,
                );
            })),
        );
    }
    panel = panel.child(grid);

    // Where the pattern is shown.
    panel = panel.child(
        div()
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Display Location"),
    );
    let mut locations = div().flex().flex_col().gap(px(4.));
    for location in Location::ALL {
        let active = state.location == location;
        locations = locations.child(
            Button::new(SharedString::from(format!(
                "test-pattern-location-{}",
                location.label().replace(' ', "-")
            )))
            .label(location.label())
            .ghost()
            .xsmall()
            .when(active, |button| button.primary())
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    crate::ui::messages::Message::TestPattern(Message::SetLocation(location)),
                    window,
                    cx,
                );
            })),
        );
    }
    panel = panel.child(locations);

    // The levels of the ramps. They only matter for the ramps, so they are
    // shown only for them: a level the operator cannot see the effect of is
    // a level they will not change.
    if matches!(state.pattern, Some(Pattern::BlackLevel(_))) {
        panel = panel.child(level_row("Black Level", state.black_level, 0, true, cx));
    }
    if matches!(state.pattern, Some(Pattern::WhiteLevel(_))) {
        panel = panel.child(level_row("White Level", state.white_level, 255, false, cx));
    }

    // The show and hide.
    panel = panel.child(
        div().flex_1().child(
            Button::new("test-pattern-toggle")
                .label(if showing { "Hide" } else { "Show" })
                .primary()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::TestPattern(Message::ToggleShow),
                        window,
                        cx,
                    );
                })),
        ),
    );

    panel
}

/// The row that sets the level of a ramp.
fn level_row(
    label: &'static str,
    level: u8,
    default: u8,
    black: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(format!("{level}")),
        );
    for step in [-32i32, -8, 8, 32] {
        let next = (level as i32 + step).clamp(0, 255) as u8;
        let changed = next != level;
        row = row.child(
            Button::new(SharedString::from(format!(
                "test-pattern-level-{label}-{step}"
            )))
            .label(match step {
                -32 => "--",
                -8 => "-",
                8 => "+",
                _ => "++",
            })
            .ghost()
            .xsmall()
            .when(!changed, |button| button.disabled(true))
            .on_click(cx.listener(move |w, _, window, cx| {
                let message = if black {
                    Message::BlackLevelChanged(next)
                } else {
                    Message::WhiteLevelChanged(next)
                };
                w.handle_message(
                    crate::ui::messages::Message::TestPattern(message),
                    window,
                    cx,
                );
            })),
        );
    }
    let _ = default;
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pattern is on the stage screens as well as the audience ones
    /// when the operator asks for every screen.
    #[test]
    fn a_pattern_on_every_screen_reaches_both() {
        let state = TestPatternState {
            pattern: Some(Pattern::ColorBars),
            location: Location::AllScreens,
            ..Default::default()
        };
        assert_eq!(
            state.pattern_for(crate::ui::stage::STAGE_SCREEN_ID),
            Some(Pattern::ColorBars)
        );
        assert_eq!(
            state.pattern_for(crate::ui::output::AUDIENCE_SCREEN_ID),
            Some(Pattern::ColorBars)
        );
    }

    /// A pattern for the audience screens stays off the stage.
    ///
    /// The stage is the speaker's own screen: a pattern there would be in
    /// the speaker's eyes all through the service.
    #[test]
    fn a_pattern_for_the_audience_stays_off_the_stage() {
        let state = TestPatternState {
            pattern: Some(Pattern::ColorBars),
            location: Location::AudienceScreens,
            ..Default::default()
        };
        assert_eq!(state.pattern_for(crate::ui::stage::STAGE_SCREEN_ID), None);
        assert_eq!(
            state.pattern_for(crate::ui::output::AUDIENCE_SCREEN_ID),
            Some(Pattern::ColorBars)
        );
    }

    /// No pattern means nothing is drawn over the picture.
    #[test]
    fn no_pattern_leaves_the_picture_alone() {
        let state = TestPatternState::default();
        assert_eq!(
            state.pattern_for(crate::ui::output::AUDIENCE_SCREEN_ID),
            None
        );
    }
}
