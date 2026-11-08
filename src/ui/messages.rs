//! Root message vocabulary: one enum the shell dispatches, with a child
//! message per feature wrapped in a single variant.
//!
//! Each feature module owns its own `Message` and exposes an `update`
//! function; the root `Message` wraps them so a view can reach any
//! feature with one enum.

use gpui_kit::Point;

/// The root message handled by `MainWindow::handle_message`.
#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    EscapePressed,
    /// Fired once after startup: applies persisted settings that have side
    /// effects (auto-starting the HTTP/OSC remote-control listeners).
    StartupConfig,
    DismissError,
    SwitchMode(ViewMode),
    SwitchInspectorTab(InspectorTab),
    OpenTheme,
    OpenTextEditor,
    FocusSearch,
    ToggleEditMode,
    SelectLeftSection(SidebarTab),
    SelectRightDockTab(RightDockTab),
    ToggleMediaBin,
    BackToList,
    Quit,

    SearchQueryChanged(String),
    /// Open a specific slide of a presentation from a global-search result row.
    SearchResultSlide(String, usize),

    NewPresentationClicked,
    CreatePresentation,
    CancelNewPresentation,
    NewPresentationNameChanged(String),
    SelectPresentation(String),
    OpenPresentation(String),
    RenamePresentation,
    RenamePresentationChanged(String),
    DeletePresentationClicked(String),
    ConfirmDeletePresentation,
    CancelDelete,

    PresentingSelectSlide(usize),
    PresentingSelectLastSlide,
    PresentingNextSlide,
    PresentingPrevSlide,
    ShowSlidesCursorMoved(Point<f32>),
    ShowSlideContextMenu(usize),
    ShowSlideGroupSubmenu,
    ShowSlideCueSubmenu,
    ShowSlideTransitionSubmenu,
    HideSlideContextMenu,
    ShowRailContextMenu(RailContextTarget),
    HideRailContextMenu,
    RailCursorMoved(Point<f32>),
    DuplicatePresentation(String),
    EditPresentation(String),
    ExportPresentation(String),
    SongToPresentation(String),
    AddSlideCue(usize, crate::domain::Cue),
    ClearSlideCues(usize),
    SetSlideTransition(usize, crate::domain::Transition),
    EditSlide(usize),
    SetSlideGridCols(usize),
    SetSlideViewMode(SlideViewMode),
    JumpToGroup(String),
    SetGlobalTransition(crate::domain::Transition),
    AnimationTick,

    StartTimer,
    StopTimer,
    ResetTimer,
    /// Extend (or arm) the stage countdown by the given number of seconds.
    CountdownAdd(u64),
    CountdownClear,

    ToggleNdi,
    NdiSendCurrent,
    NdiBlackScreen,
    ClearOutput,
    ClearAll,
    ClearSlide,
    ClearMedia,
    ClearProps,
    ClearMessages,
    ClearAudio,

    SelectPreviewScreen(String),
    ToggleLooksMatrixModal,
    SelectLook(String),
    CreateLook(String),
    DeleteLook(String),
    LookNameInputChanged(String),
    ToggleLookScreenLayer(String, String, LookLayerType),
    SetLookScreenTheme(String, String, Option<String>),
    SendLiveMessage(String),
    DismissLiveMessage,
    LiveMessageInputChanged(String),
    AnnouncementInputChanged(String),

    Ndi(crate::ui::ndi::Message),

    ImageFilePicked(Option<String>),
    VideoFilePicked(Option<String>),

    ToggleStageDisplay,
    /// Open (or close) the dedicated stage-display OS window.
    ToggleStageWindow,
    ClockTick,
    Stage(crate::ui::stage::Message),

    SwitchSidebarTab(SidebarTab),
    Library(crate::ui::library::Message),

    Themes(crate::ui::themes::Message),

    Slides(crate::ui::slides::Message),
    Layers(crate::ui::layers::Message),
    Typography(crate::ui::typography::Message),

    ToggleShortcutsOverlay,
    ToggleAbout,
    Undo,
    Redo,
    ToggleReduceMotion,
    Playlist(crate::ui::playlist::Message),
    Songs(crate::ui::songs::Message),
    Bible(crate::ui::bible::Message),
    Calendar(crate::ui::calendar::Message),
    CaptureSettings(crate::ui::capture_settings::Message),
    ClearGroups(crate::ui::clear_groups::Message),
    CueInspector(crate::ui::cue_inspector::Message),
    Groups(crate::ui::groups::Message),
    TestPattern(crate::ui::test_pattern::Message),
    Timeline(crate::ui::timeline::Message),

    VideoFrameTick,
    /// Periodic audio tick (position readout + queue auto-advance).
    AudioTick,
    Video(crate::ui::video::Message),

    OpenOutputWindow,
    CloseOutputWindow,
    ToggleOutputWindow,
    OutputWindowOpened,
    ToggleOutputBlackScreen,
    ToggleOutputSettings,
    OutputScreenXChanged(String),
    OutputScreenYChanged(String),
    OutputAutoFullscreenToggled(bool),
    OutputFullscreenToggled,

    Audio(crate::ui::audio::Message),

    Output(crate::ui::output::Message),

    ImportExport(crate::ui::import_export::Message),

    Props(crate::ui::props::Message),
    SetMask(crate::domain::Mask),
    TogglePropsPanel,

    ToggleTriggersPanel,
    Triggers(crate::ui::triggers::Message),
    TriggerFired(crate::triggers::Action),

    Recording(crate::ui::recording::Message),
    ToggleRecordingPanel,

    // Editor shell commands.
    Zoom(ZoomCommand),
    Arrange(ArrangeCommand),
    Insert(InsertCommand),
    TextStyle(TextStyleCommand),
    ToggleRulers,
    ToggleSlideGuides,
    ToggleThemeGuides,
    CanvasBeginTextEdit(usize),
    CopyObject,
    PasteObject,
    DeleteObject,
    DuplicateObject,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    NudgeLeftFast,
    NudgeRightFast,
    NudgeUpFast,
    NudgeDownFast,
    OpenSettings,
    OpenSettingsTab(SettingsTab),
}

/// How much of the slide the editor shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomCommand {
    /// One slide pixel is one screen pixel.
    ActualSize,
    /// The slide fits its workspace.
    FitCanvas,
    /// The slide fits the canvas.
    FitSlide,
    /// A step closer.
    In,
    /// A step farther.
    Out,
}

/// Where the selected objects sit on the slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrangeCommand {
    /// Line the objects up.
    Align(crate::domain::AlignMode),
    /// Move through the draw order.
    Reorder(crate::domain::Reorder),
    /// Mirror left to right.
    FlipHorizontal,
    /// Mirror top to bottom.
    FlipVertical,
    /// Stop the objects from moving.
    Lock,
    /// Free every object on the slide.
    UnlockAll,
}

/// What goes on the slide the operator is editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertCommand {
    /// A text object.
    Text,
    /// A shape object.
    Shape,
    /// A media object waiting for a file.
    Media,
    /// A web-page stand-in.
    Web,
    /// A countdown stand-in.
    Timer,
    /// A playback-marker stand-in.
    PlaybackMarker,
    /// A screen-preview stand-in.
    ScreenPreview,
    /// A stage-message stand-in.
    StageMessage,
    /// A timecode stand-in.
    Timecode,
    /// A chord-chart stand-in.
    ChordChart,
}

/// A command from the Text menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextStyleCommand {
    /// Copy the text style.
    CopyStyle,
    /// Paste a copied style.
    PasteStyle,
    /// Apply the style to every slide.
    ApplyToAllSlides,
    /// Toggle bold.
    Bold,
    /// Toggle italic.
    Italic,
    /// Toggle underline.
    Underline,
    /// Toggle strikethrough.
    Strikethrough,
    /// A step larger.
    Bigger,
    /// A step smaller.
    Smaller,
    /// More letter spacing.
    Loosen,
    /// Less letter spacing.
    Tighten,
    /// Reset letter spacing.
    SpacingDefault,
    /// Set the casing.
    Capitalization(crate::domain::TextTransform),
    /// Set the baseline.
    Baseline(crate::domain::Baseline),
    /// Set the ligatures.
    Ligatures(crate::domain::Ligatures),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlideViewMode {
    Easy,
    #[default]
    Grid,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Edit,
    Show,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Text,
    Slide,
    Theme,
    Layers,
}

/// Which tab is shown in the right dock while in Show mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightDockTab {
    #[default]
    ShowControls,
    Props,
    Messages,
    Triggers,
    Audio,
    Timers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookLayerType {
    Media,
    Slide,
    Props,
    Messages,
    Mask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarTab {
    #[default]
    Presentations,
    Playlists,
    Library,
    Songs,
    Bible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RailContextTarget {
    Presentation(String),
    Playlist(String),
    LibraryAsset(String),
    Song(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Audio,
    Devices,
    GroupsAndLabels,
    Import,
    Inputs,
    Network,
    Screens,
    Updates,
    Media,
    SlideLabels,
    Transitions,
    Advanced,
}

impl SettingsTab {
    /// Every settings page in menu order.
    pub const ALL: [SettingsTab; 13] = [
        SettingsTab::General,
        SettingsTab::Audio,
        SettingsTab::Devices,
        SettingsTab::GroupsAndLabels,
        SettingsTab::Import,
        SettingsTab::Inputs,
        SettingsTab::Network,
        SettingsTab::Screens,
        SettingsTab::Updates,
        SettingsTab::Media,
        SettingsTab::SlideLabels,
        SettingsTab::Transitions,
        SettingsTab::Advanced,
    ];

    /// The name the settings window shows.
    pub fn label(&self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Audio => "Audio",
            SettingsTab::Devices => "Devices",
            SettingsTab::GroupsAndLabels => "Groups & Labels",
            SettingsTab::Import => "Import",
            SettingsTab::Inputs => "Inputs",
            SettingsTab::Network => "Network",
            SettingsTab::Screens => "Screens",
            SettingsTab::Updates => "Updates",
            SettingsTab::Media => "Media",
            SettingsTab::SlideLabels => "Slide Labels",
            SettingsTab::Transitions => "Transitions",
            SettingsTab::Advanced => "Advanced",
        }
    }
}

impl From<crate::ui::playlist::Message> for Message {
    fn from(msg: crate::ui::playlist::Message) -> Self {
        Message::Playlist(msg)
    }
}

impl From<crate::ui::songs::Message> for Message {
    fn from(msg: crate::ui::songs::Message) -> Self {
        Message::Songs(msg)
    }
}

impl From<crate::ui::bible::Message> for Message {
    fn from(msg: crate::ui::bible::Message) -> Self {
        Message::Bible(msg)
    }
}

impl From<crate::ui::props::Message> for Message {
    fn from(msg: crate::ui::props::Message) -> Self {
        Message::Props(msg)
    }
}

impl From<crate::ui::audio::Message> for Message {
    fn from(msg: crate::ui::audio::Message) -> Self {
        Message::Audio(msg)
    }
}

impl From<crate::ui::library::Message> for Message {
    fn from(msg: crate::ui::library::Message) -> Self {
        Message::Library(msg)
    }
}

impl From<crate::ui::recording::Message> for Message {
    fn from(msg: crate::ui::recording::Message) -> Self {
        Message::Recording(msg)
    }
}

impl From<crate::ui::triggers::Message> for Message {
    fn from(msg: crate::ui::triggers::Message) -> Self {
        Message::Triggers(msg)
    }
}

impl From<crate::ui::import_export::Message> for Message {
    fn from(msg: crate::ui::import_export::Message) -> Self {
        Message::ImportExport(msg)
    }
}

impl From<crate::ui::stage::Message> for Message {
    fn from(msg: crate::ui::stage::Message) -> Self {
        Message::Stage(msg)
    }
}

impl From<crate::ui::video::Message> for Message {
    fn from(msg: crate::ui::video::Message) -> Self {
        Message::Video(msg)
    }
}

impl From<crate::ui::ndi::Message> for Message {
    fn from(msg: crate::ui::ndi::Message) -> Self {
        Message::Ndi(msg)
    }
}

impl From<crate::ui::output::Message> for Message {
    fn from(msg: crate::ui::output::Message) -> Self {
        Message::Output(msg)
    }
}

impl From<crate::ui::themes::Message> for Message {
    fn from(msg: crate::ui::themes::Message) -> Self {
        Message::Themes(msg)
    }
}

impl From<crate::ui::slides::Message> for Message {
    fn from(msg: crate::ui::slides::Message) -> Self {
        Message::Slides(msg)
    }
}

impl From<crate::ui::layers::Message> for Message {
    fn from(msg: crate::ui::layers::Message) -> Self {
        Message::Layers(msg)
    }
}

impl From<crate::ui::typography::Message> for Message {
    fn from(msg: crate::ui::typography::Message) -> Self {
        Message::Typography(msg)
    }
}

impl From<crate::ui::calendar::Message> for Message {
    fn from(msg: crate::ui::calendar::Message) -> Self {
        Message::Calendar(msg)
    }
}

impl From<crate::ui::capture_settings::Message> for Message {
    fn from(msg: crate::ui::capture_settings::Message) -> Self {
        Message::CaptureSettings(msg)
    }
}

impl From<crate::ui::clear_groups::Message> for Message {
    fn from(msg: crate::ui::clear_groups::Message) -> Self {
        Message::ClearGroups(msg)
    }
}

impl From<crate::ui::cue_inspector::Message> for Message {
    fn from(msg: crate::ui::cue_inspector::Message) -> Self {
        Message::CueInspector(msg)
    }
}

impl From<crate::ui::groups::Message> for Message {
    fn from(msg: crate::ui::groups::Message) -> Self {
        Message::Groups(msg)
    }
}

impl From<crate::ui::test_pattern::Message> for Message {
    fn from(msg: crate::ui::test_pattern::Message) -> Self {
        Message::TestPattern(msg)
    }
}

impl From<crate::ui::timeline::Message> for Message {
    fn from(msg: crate::ui::timeline::Message) -> Self {
        Message::Timeline(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slide_view_mode_defaults_to_grid() {
        assert_eq!(SlideViewMode::default(), SlideViewMode::Grid);
    }

    #[test]
    fn test_slide_view_mode_equality() {
        assert_ne!(SlideViewMode::Grid, SlideViewMode::Table);
    }
}
