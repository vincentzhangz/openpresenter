//! The application menu bar.
//!
//! On a Mac the app's commands live in the menu bar, and the menu bar is
//! the map of what the app can do: an operator who opens the Edit menu sees
//! everything the Edit menu can reach, in the order the app does it. The
//! menu is therefore built from the same actions the keyboard shortcuts use,
//! so a command that is in the menu is a command that works.
//!
//! The menu is rebuilt when the state it shows changes — which capitalization
//! is in use, which slide view the editor is in — so a check mark in the menu
//! is the truth about the slide, not a leftover from when the menu was built.

use gpui_kit::{Menu, MenuItem, OsAction};

use crate::domain::{Baseline, Ligatures, TextTransform};
use crate::ui::actions;

/// The state the menu shows.
///
/// The menu is a picture of what the app is doing, so it carries the choices
/// that are currently in use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuState {
    /// The casing the edited text is drawn with.
    pub capitalization: TextTransform,
    /// Where the edited text sits on its line.
    pub baseline: Baseline,
    /// Whether the edited text joins its letters together.
    pub ligatures: Ligatures,
    /// The family the edited text is drawn in.
    pub font_family: String,
    /// The view the slide list is drawn in.
    pub slide_view: SlideViewChoice,
    /// Whether the running service loops at the end.
    pub continuous_playlist: bool,
    /// Whether the editor is on the screen.
    pub editor_visible: bool,
    /// Whether the media bin is on the screen.
    pub media_bin_visible: bool,
}

/// The three ways the slide list can be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlideViewChoice {
    /// A short list, one row per slide.
    Easy,
    /// A grid of thumbnails.
    #[default]
    Grid,
    /// A table of slides and their details.
    Table,
}

/// Build the menu bar.
///
/// The order is the one the operator expects: what is made and saved, what is
/// changed, what goes on the slide, what the audience sees, what is on the
/// screen, and finally the windows.
pub fn build(state: &MenuState) -> Vec<Menu> {
    vec![
        application_menu(state),
        file_menu(),
        edit_menu(),
        editor_menu(state),
        presentation_menu(),
        view_menu(state),
        window_menu(),
        help_menu(),
    ]
}

/// The application's own menu.
fn application_menu(_state: &MenuState) -> Menu {
    let mut items = vec![
        MenuItem::action("About OpenPresenter", actions::AboutApp),
        MenuItem::separator(),
        // The settings submenu names every page, so the operator can go
        // straight to the one they want instead of opening the window and
        // looking for it.
        MenuItem::submenu(Menu::new("Settings").items([
            MenuItem::action("General…", actions::OpenSettingsGeneral),
            MenuItem::action("Audio…", actions::OpenSettingsAudio),
            MenuItem::action("Devices…", actions::OpenSettingsDevices),
            MenuItem::action("Groups & Labels…", actions::OpenSettingsGroups),
            MenuItem::action("Import…", actions::OpenSettingsImport),
            MenuItem::action("Inputs…", actions::OpenSettingsInputs),
            MenuItem::action("Network…", actions::OpenSettingsNetwork),
            MenuItem::action("Screens…", actions::OpenSettingsScreens),
            MenuItem::action("Updates…", actions::OpenSettingsUpdates),
            MenuItem::action("Media…", actions::OpenSettingsMedia),
            MenuItem::action("Slide Labels…", actions::OpenSettingsSlideLabels),
            MenuItem::separator(),
            MenuItem::action("Transitions…", actions::OpenSettingsTransitions),
            MenuItem::action("Advanced…", actions::OpenSettingsAdvanced),
        ])),
        MenuItem::action("Key Mappings…", actions::OpenKeyMappings),
        MenuItem::action("Groups & Labels…", actions::OpenGroupsSettings),
        MenuItem::separator(),
    ];
    // The standard macOS items. They are here because an operator expects
    // them under the app's name, and because the shortcut in the menu is
    // the shortcut the app acts on. Hiding an app is a macOS idea, so the
    // items are only there on macOS.
    #[cfg(target_os = "macos")]
    {
        items.push(MenuItem::action("Hide OpenPresenter", actions::HideApp));
        items.push(MenuItem::action("Hide Others", actions::HideOtherApps));
        items.push(MenuItem::action("Show All", actions::ShowAllApps));
        items.push(MenuItem::separator());
    }
    items.push(MenuItem::action("Quit OpenPresenter", actions::QuitApp));
    Menu::new("OpenPresenter").items(items)
}

/// What the operator makes and what the operator sends out.
fn file_menu() -> Menu {
    Menu::new("File").items([
        MenuItem::action("New Presentation", actions::NewPresentation),
        MenuItem::action("New Playlist", actions::NewPlaylist),
        MenuItem::action("New Library", actions::NewLibrary),
        MenuItem::separator(),
        MenuItem::submenu(Menu::new("Import").items([
            MenuItem::action("File…", actions::ImportFile),
            MenuItem::action("Media…", actions::ImportMedia),
            MenuItem::action("Text from Clipboard…", actions::ImportTextFromClipboard),
            MenuItem::action("PowerPoint as Objects and Text…", actions::ImportPowerPoint),
            MenuItem::action("PowerPoint as Images…", actions::ImportPowerPointAsImages),
            MenuItem::action(
                "PowerPoint as Presentation…",
                actions::ImportPowerPointAsPresentation,
            ),
            MenuItem::action("Library…", actions::ImportLibrary),
        ])),
        MenuItem::submenu(Menu::new("Export").items([
            MenuItem::action("Presentation…", actions::ExportPresentation),
            MenuItem::action("Slide Images…", actions::ExportSlideImages),
            MenuItem::action("Playlist…", actions::ExportPlaylist),
            MenuItem::action("Text…", actions::ExportText),
            MenuItem::action("Themes…", actions::ExportThemes),
            MenuItem::action("Presentation as PDF…", actions::ExportPdf),
        ])),
        MenuItem::separator(),
        MenuItem::action("Print…", actions::PrintPresentation),
    ])
}

/// What the operator changes.
fn edit_menu() -> Menu {
    Menu::new("Edit").items([
        MenuItem::action("Undo", actions::Undo),
        MenuItem::action("Redo", actions::Redo),
        MenuItem::separator(),
        MenuItem::os_action("Cut", actions::Cut, OsAction::Cut),
        MenuItem::os_action("Copy", actions::CopyObject, OsAction::Copy),
        MenuItem::os_action("Paste", actions::PasteObject, OsAction::Paste),
        MenuItem::action("Copy Text Style", actions::CopyTextStyle),
        MenuItem::action("Paste Text Style", actions::PasteTextStyle),
        MenuItem::separator(),
        MenuItem::os_action("Select All", actions::SelectAll, OsAction::SelectAll),
        MenuItem::action("Deselect All", actions::DeselectAll),
        MenuItem::action("Duplicate", actions::DuplicateObject),
        MenuItem::action("Delete", actions::DeleteObject),
        MenuItem::separator(),
        MenuItem::submenu(Menu::new("Transformations").items([
            MenuItem::action("Make Upper Case", actions::TransformUpperCase),
            MenuItem::action("Make Lower Case", actions::TransformLowerCase),
            MenuItem::action("Capitalize", actions::TransformCapitalize),
        ])),
        MenuItem::separator(),
        MenuItem::action("Search Library", actions::SearchLibrary),
    ])
}

/// What goes on the slide, and where it sits.
fn editor_menu(state: &MenuState) -> Menu {
    Menu::new("Editor").items([
        MenuItem::action("New Slide", actions::NewSlide),
        MenuItem::separator(),
        MenuItem::submenu(insert_menu()),
        MenuItem::submenu(arrangement_menu()),
        MenuItem::submenu(text_menu(state)),
        MenuItem::submenu(zoom_menu()),
        MenuItem::separator(),
        MenuItem::action("Show Rulers", actions::ShowRulers),
        MenuItem::action("Show Slide Guides", actions::ShowSlideGuides),
        MenuItem::action("Show Theme Guides", actions::ShowThemeGuides),
    ])
}

/// The objects that can go on a slide.
fn insert_menu() -> Menu {
    Menu::new("Insert").items([
        MenuItem::action("Text", actions::InsertText),
        MenuItem::action("Shape", actions::InsertShape),
        MenuItem::action("Media", actions::InsertMedia),
        MenuItem::action("Web", actions::InsertWeb),
        MenuItem::action("Timer", actions::InsertTimer),
        MenuItem::action("Playback Marker", actions::InsertPlaybackMarker),
        MenuItem::action("Screen Preview", actions::InsertScreenPreview),
        MenuItem::action("Stage Message", actions::InsertStageMessage),
        MenuItem::action("Timecode", actions::InsertTimecode),
        MenuItem::action("Chord Chart", actions::InsertChordChart),
        MenuItem::separator(),
    ])
}

/// Where the selected objects sit on the slide.
fn arrangement_menu() -> Menu {
    Menu::new("Arrangement").items([
        MenuItem::submenu(Menu::new("Align Objects").items([
            MenuItem::action("Left", actions::AlignLeft),
            MenuItem::action("Center", actions::AlignCenter),
            MenuItem::action("Right", actions::AlignRight),
            MenuItem::separator(),
            MenuItem::action("Top", actions::AlignTop),
            MenuItem::action("Middle", actions::AlignMiddle),
            MenuItem::action("Bottom", actions::AlignBottom),
        ])),
        MenuItem::separator(),
        MenuItem::action("Bring Forward", actions::BringForward),
        MenuItem::action("Bring to Front", actions::BringToFront),
        MenuItem::action("Send Backward", actions::SendBackward),
        MenuItem::action("Send to Back", actions::SendToBack),
        MenuItem::separator(),
        MenuItem::action("Flip Horizontally", actions::FlipHorizontally),
        MenuItem::action("Flip Vertically", actions::FlipVertically),
        MenuItem::separator(),
        MenuItem::action("Lock", actions::LockObject),
        MenuItem::action("Unlock All", actions::UnlockAllObjects),
    ])
}

/// How the text is drawn.
fn text_menu(state: &MenuState) -> Menu {
    let capitalization = Menu::new("Capitalization").items([
        MenuItem::action("None", actions::TextCapitalizationNone)
            .checked(state.capitalization == TextTransform::None),
        MenuItem::action("All Caps", actions::TextCapitalizationAllCaps)
            .checked(state.capitalization == TextTransform::Uppercase),
        MenuItem::action("Small Caps", actions::TextCapitalizationSmallCaps)
            .checked(state.capitalization == TextTransform::SmallCaps),
        MenuItem::action("Title Case", actions::TextCapitalizationTitleCase)
            .checked(state.capitalization == TextTransform::TitleCase),
        MenuItem::action("Start Case", actions::TextCapitalizationStartCase)
            .checked(state.capitalization == TextTransform::StartCase),
        MenuItem::action("Lower Case", actions::TextCapitalizationLowerCase)
            .checked(state.capitalization == TextTransform::Lowercase),
    ]);

    let baseline = Menu::new("Baseline").items([
        MenuItem::action("Default", actions::TextBaselineDefault)
            .checked(state.baseline == Baseline::Default),
        MenuItem::action("Raise", actions::TextBaselineRaise)
            .checked(state.baseline == Baseline::Raise),
        MenuItem::action("Lower", actions::TextBaselineLower)
            .checked(state.baseline == Baseline::Lower),
        MenuItem::action("Use Superscript", actions::TextBaselineSuperscript)
            .checked(state.baseline == Baseline::Superscript),
        MenuItem::action("Use Subscript", actions::TextBaselineSubscript)
            .checked(state.baseline == Baseline::Subscript),
    ]);

    // The font the text is drawn in. The families are the ones the Layers
    // tab offers, so the menu and the panel pick from the same list.
    let font = Menu::new("Font").items([
        MenuItem::action("Arial", actions::TextFontArial).checked(state.font_family == "Arial"),
        MenuItem::action("Georgia", actions::TextFontGeorgia)
            .checked(state.font_family == "Georgia"),
        MenuItem::action("Impact", actions::TextFontImpact).checked(state.font_family == "Impact"),
        MenuItem::action("Courier New", actions::TextFontCourierNew)
            .checked(state.font_family == "Courier New"),
    ]);

    let ligatures = Menu::new("Ligature").items([
        MenuItem::action("Use Default", actions::TextLigaturesDefault)
            .checked(state.ligatures == Ligatures::Default),
        MenuItem::action("Use None", actions::TextLigaturesNone)
            .checked(state.ligatures == Ligatures::None),
    ]);

    Menu::new("Text").items([
        MenuItem::action("Bold", actions::TextBold),
        MenuItem::action("Italic", actions::TextItalic),
        MenuItem::action("Underline", actions::TextUnderline),
        MenuItem::action("Strikethrough", actions::TextStrikethrough),
        MenuItem::separator(),
        MenuItem::action("Bigger", actions::TextBigger),
        MenuItem::action("Smaller", actions::TextSmaller),
        MenuItem::separator(),
        MenuItem::submenu(Menu::new("Character Spacing").items([
            MenuItem::action("Default", actions::TextSpacingDefault),
            MenuItem::action("Loosen", actions::TextLoosen),
            MenuItem::action("Tighten", actions::TextTighten),
        ])),
        MenuItem::submenu(font),
        MenuItem::submenu(capitalization),
        MenuItem::submenu(baseline),
        MenuItem::submenu(ligatures),
        MenuItem::separator(),
        MenuItem::action(
            "Apply Text Style to All Slides",
            actions::TextApplyStyleToAllSlides,
        ),
        MenuItem::action("Copy Text Style", actions::CopyTextStyle),
        MenuItem::action("Paste Text Style", actions::PasteTextStyle),
    ])
}

/// How much of the slide the editor shows.
fn zoom_menu() -> Menu {
    Menu::new("Zoom").items([
        MenuItem::action("Actual Size", actions::ZoomActualSize),
        MenuItem::action("Fit Canvas", actions::ZoomFitCanvas),
        MenuItem::action("Fit Slide", actions::ZoomFitSlide),
        MenuItem::separator(),
        MenuItem::action("Zoom In", actions::ZoomIn),
        MenuItem::action("Zoom Out", actions::ZoomOut),
    ])
}

/// What the audience sees.
fn presentation_menu() -> Menu {
    Menu::new("Presentation").items([
        MenuItem::action("Next Slide", actions::NextSlide),
        MenuItem::action("Previous Slide", actions::PrevSlide),
        MenuItem::separator(),
        MenuItem::action("Clear Slide", actions::ClearSlide),
        MenuItem::action("Clear Media", actions::ClearMedia),
        MenuItem::action("Clear Audio", actions::ClearAudio),
        MenuItem::action("Clear Props", actions::ClearProps),
        MenuItem::action("Clear Messages", actions::ClearMessages),
        MenuItem::action("Clear Groups", actions::ClearGroups),
        MenuItem::action("Clear Announcements", actions::ClearAnnouncements),
        MenuItem::action("Clear to Logo", actions::ClearToLogo),
        MenuItem::separator(),
        MenuItem::action("Configure Clear Groups…", actions::ConfigureClearGroups),
        MenuItem::action("Reveal in Library", actions::RevealInLibrary),
        MenuItem::separator(),
        MenuItem::action("Next Background", actions::NextBackground),
        MenuItem::action("Previous Background", actions::PreviousBackground),
    ])
}

/// Which part of the app is on the screen.
fn view_menu(state: &MenuState) -> Menu {
    Menu::new("View").items([
        MenuItem::action("Presentation Editor", actions::ViewEditor).checked(state.editor_visible),
        MenuItem::action("Show", actions::ViewShow),
        MenuItem::separator(),
        MenuItem::action("Stage", actions::ViewStage),
        MenuItem::action("Media Bin", actions::ViewMediaBin).checked(state.media_bin_visible),
        MenuItem::action("Props", actions::ViewProps),
        MenuItem::action("Messages", actions::ViewMessages),
        MenuItem::action("Timers", actions::ViewTimers),
        MenuItem::action("Timecode", actions::ViewTimecode),
        MenuItem::action("Test Patterns", actions::OpenTestPatterns),
        MenuItem::action("Bibles", actions::ViewBibles),
        MenuItem::action("Cue Inspector", actions::OpenCueInspector),
        MenuItem::separator(),
        MenuItem::action("Action Palette", actions::ViewActionPalette),
        MenuItem::action("Macros", actions::ViewMacros),
        MenuItem::action("Continuous Playlist", actions::ViewContinuousPlaylist)
            .checked(state.continuous_playlist),
        MenuItem::action("Timeline", actions::ViewTimeline),
        MenuItem::action("Service Calendar", actions::ViewCalendar),
        MenuItem::separator(),
        MenuItem::submenu(
            Menu::new("Slide View").items([
                MenuItem::action("Easy View", actions::ViewSlideEasy)
                    .checked(state.slide_view == SlideViewChoice::Easy),
                MenuItem::action("Grid View", actions::ViewSlideGrid)
                    .checked(state.slide_view == SlideViewChoice::Grid),
                MenuItem::action("Table View", actions::ViewSlideTable)
                    .checked(state.slide_view == SlideViewChoice::Table),
            ]),
        ),
    ])
}

/// The windows the app has open.
fn window_menu() -> Menu {
    Menu::new("Window").items([
        MenuItem::action("Minimize", actions::MinimizeWindow),
        MenuItem::action("Zoom", actions::ZoomWindow),
        MenuItem::separator(),
        MenuItem::action("Bring All to Front", actions::BringAllToFront),
    ])
}

/// Where the operator finds help.
fn help_menu() -> Menu {
    Menu::new("Help").items([
        MenuItem::action("OpenPresenter Help", actions::OpenHelp),
        MenuItem::separator(),
        MenuItem::action("Reset All Settings", actions::ResetAllSettings),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every menu the app shows is reachable: a menu that is built but never
    /// opened is a command nobody can use.
    #[test]
    fn every_menu_has_a_name() {
        for menu in build(&MenuState::default()) {
            assert!(!menu.name.is_empty());
        }
    }

    /// The commands the operator reaches for during a show are on the
    /// presentation menu, where the operator expects them.
    #[test]
    fn the_presentation_menu_carries_the_clears() {
        let menus = build(&MenuState::default());
        let presentation = menus
            .iter()
            .find(|menu| menu.name.as_ref() == "Presentation")
            .expect("the app has a presentation menu");
        let names: Vec<&str> = presentation
            .items
            .iter()
            .map(|item| match item {
                MenuItem::Action { name, .. } => name.as_ref(),
                _ => "",
            })
            .collect();
        for expected in [
            "Clear Slide",
            "Clear Media",
            "Clear Audio",
            "Clear Props",
            "Clear Messages",
            "Clear Groups",
        ] {
            assert!(
                names.contains(&expected),
                "the presentation menu is missing {expected}"
            );
        }
    }

    /// The check mark in the menu is the truth about the slide: the casing
    /// that is in use is the one that is ticked.
    #[test]
    fn the_casing_in_use_is_ticked() {
        let state = MenuState {
            capitalization: TextTransform::SmallCaps,
            ..Default::default()
        };
        let menus = build(&state);
        let editor = menus
            .iter()
            .find(|menu| menu.name.as_ref() == "Editor")
            .expect("the app has an editor menu");
        let text = editor
            .items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu(menu) if menu.name.as_ref() == "Text" => Some(menu),
                _ => None,
            })
            .expect("the editor menu has a text menu");
        let capitalization = text
            .items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu(menu) if menu.name.as_ref() == "Capitalization" => Some(menu),
                _ => None,
            })
            .expect("the text menu has a capitalization menu");
        let ticked: Vec<&str> = capitalization
            .items
            .iter()
            .filter(|item| item.is_checked())
            .map(|item| match item {
                MenuItem::Action { name, .. } => name.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(ticked, vec!["Small Caps"]);
    }

    /// The names of the items in a submenu.
    fn submenu_items(menus: &[Menu], menu_name: &str, submenu_name: &str) -> Vec<String> {
        let submenu = find_submenu(menus, menu_name, submenu_name);
        submenu
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { name, .. } => Some(name.to_string()),
                _ => None,
            })
            .collect()
    }

    /// A submenu of a menu, by name.
    fn find_submenu<'a>(menus: &'a [Menu], menu_name: &str, submenu_name: &str) -> &'a Menu {
        let menu = menus
            .iter()
            .find(|menu| menu.name.as_ref() == menu_name)
            .expect("the app has the menu");
        menu.items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu(menu) if menu.name.as_ref() == submenu_name => Some(menu),
                _ => None,
            })
            .expect("the menu has the submenu")
    }

    /// The Transformations menu rewrites the words, so it is in the Edit
    /// menu where the operator changes what is written.
    #[test]
    fn the_edit_menu_carries_the_transformations() {
        let menus = build(&MenuState::default());
        let names = submenu_items(&menus, "Edit", "Transformations");
        assert_eq!(
            names,
            vec!["Make Upper Case", "Make Lower Case", "Capitalize"]
        );
    }

    /// The font the text is drawn in is in the menu, and the menu offers
    /// the same families the Layers tab offers: a family the operator finds
    /// in one is a family they find in the other.
    #[test]
    fn the_font_menu_matches_the_families_the_app_offers() {
        let menus = build(&MenuState::default());
        let font = find_submenu(&menus, "Editor", "Text");
        let font = font
            .items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu(menu) if menu.name.as_ref() == "Font" => Some(menu),
                _ => None,
            })
            .expect("the text menu has a font menu");
        let offered: Vec<&str> = font
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { name, .. } => Some(name.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(offered, crate::ui::typography::FONT_PRESETS.to_vec());
    }

    /// Every action the menu builds has a dispatch route: a menu item whose
    /// action is not in `crate::ui::actions::menu_actions` is a command the
    /// operator can click that does nothing.
    #[test]
    fn every_menu_action_has_a_dispatch_route() {
        use std::collections::BTreeSet;
        fn collect_actions(menus: &[Menu], into: &mut BTreeSet<String>) {
            for menu in menus {
                for item in &menu.items {
                    match item {
                        MenuItem::Action { action, .. } => {
                            let name = action.name();
                            let bare = name.rsplit("::").next().unwrap_or(name).to_string();
                            into.insert(bare);
                        }
                        MenuItem::Submenu(menu) => {
                            collect_actions(std::slice::from_ref(menu), into)
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut menu_names = BTreeSet::new();
        collect_actions(&build(&MenuState::default()), &mut menu_names);
        let routed: BTreeSet<String> = crate::ui::actions::menu_actions()
            .into_iter()
            .map(|action| {
                let name = action.name().to_string();
                name.rsplit("::").next().unwrap_or(&name).to_string()
            })
            .collect();
        let missing: Vec<_> = menu_names.difference(&routed).cloned().collect();
        assert!(
            missing.is_empty(),
            "the menu names actions with no dispatch route: {missing:?}"
        );
    }

    /// The font in use is the one that is ticked, so the menu is the truth
    /// about the text.
    #[test]
    fn the_font_in_use_is_ticked() {
        let state = MenuState {
            font_family: "Georgia".to_string(),
            ..Default::default()
        };
        let menus = build(&state);
        let text = find_submenu(&menus, "Editor", "Text");
        let font = text
            .items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu(menu) if menu.name.as_ref() == "Font" => Some(menu),
                _ => None,
            })
            .expect("the text menu has a font menu");
        let ticked: Vec<&str> = font
            .items
            .iter()
            .filter(|item| item.is_checked())
            .map(|item| match item {
                MenuItem::Action { name, .. } => name.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(ticked, vec!["Georgia"]);
    }
}
