#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    EscapePressed,
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
    ShowSlidesCursorMoved(iced::Point),
    ShowSlideContextMenu(usize),
    ShowSlideGroupSubmenu,
    ShowSlideCueSubmenu,
    ShowSlideTransitionSubmenu,
    HideSlideContextMenu,
    ShowRailContextMenu(RailContextTarget),
    HideRailContextMenu,
    RailCursorMoved(iced::Point),
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

    Ndi(crate::ui::ndi::Message),

    ImageFilePicked(Option<String>),
    VideoFilePicked(Option<String>),

    ToggleStageDisplay,
    ClockTick,
    Stage(crate::ui::stage::Message),

    SwitchSidebarTab(SidebarTab),
    Library(crate::ui::library::Message),

    Themes(crate::ui::themes::Message),

    Slides(crate::ui::slides::Message),
    Layers(crate::ui::layers::Message),
    Typography(crate::ui::typography::Message),

    ToggleShortcutsOverlay,
    Undo,
    Redo,
    ToggleReduceMotion,
    Playlist(crate::ui::playlist::Message),
    Songs(crate::ui::songs::Message),
    Bible(crate::ui::bible::Message),

    VideoFrameTick,
    Video(crate::ui::video::Message),

    OpenOutputWindow,
    CloseOutputWindow,
    ToggleOutputWindow,
    OutputWindowOpened,
    ToggleOutputBlackScreen,
    WindowClosed(iced::window::Id),
    ToggleOutputSettings,
    OutputScreenXChanged(String),
    OutputScreenYChanged(String),
    OutputAutoFullscreenToggled(bool),
    OutputFullscreenToggled,
    OutputMonitorSizeFetched(Option<iced::Size>),

    Audio(crate::ui::audio::Message),

    Output(crate::ui::output::Message),

    ImportExport(crate::ui::import_export::Message),
    ToggleImportExportPanel,

    Props(crate::ui::props::Message),
    SetMask(crate::domain::Mask),
    TogglePropsPanel,

    ToggleTriggersPanel,
    Triggers(crate::ui::triggers::Message),
    TriggerFired(crate::triggers::Action),

    Recording(crate::ui::recording::Message),
    ToggleRecordingPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlideViewMode {
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
    Screens,
    NetworkNdi,
    Transitions,
    Advanced,
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
