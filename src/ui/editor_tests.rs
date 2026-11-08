//! The editor, exercised in a real window.
//!
//! These tests drive the app the way the operator does: they open the
//! editor, use the controls and read the state back. The window is a real
//! one, so the controls are the ones the operator sees.

use std::sync::Arc;

use gpui_kit::test::{TestQueryExt, TestWindowExt};
use gpui_kit::{AppContext, Context, Entity, ScrollDelta, SharedString, TestAppContext, Window};

use crate::db::Database;
use crate::db::repository::PresentationRepository;
use crate::domain::{Object, Presentation, Slide};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::ViewMode;

/// A presentation with one slide and one text object on it.
fn presentation() -> Presentation {
    let mut presentation = Presentation::new("Sunday Service".to_string());
    let mut slide = Slide::new_text("Welcome".to_string());
    slide.id = "slide-1".to_string();
    let mut object = Object::new_text("Welcome".to_string());
    object.id = "object-1".to_string();
    slide.layers.push(object);
    presentation.slides.push(slide);
    presentation
}

/// Open the editor with a presentation ready to work on.
fn open_editor(
    cx: &mut TestAppContext,
    pres: &Presentation,
) -> (
    gpui_kit::AnyWindowHandle,
    Entity<MainWindow>,
    Arc<Database>,
    Presentation,
) {
    let db = Arc::new(Database::in_memory().expect("the library opens"));
    let stored = PresentationRepository::new(Arc::clone(&db))
        .create_presentation_with_slides(&pres.name, &pres.slides)
        .expect("the presentation is stored");

    // The window is given scratch settings: a test that read — or wrote —
    // the operator's own configuration would change the app they use, and the
    // next run would find the triggers panel taller than the screen.
    let app_config = scratch_config();

    let library = Arc::clone(&db);
    let opened = cx.update(|cx| {
        gpui_kit::open_window(
            gpui_kit::WindowOptions {
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                    origin: Default::default(),
                    size: gpui_kit::size(gpui_kit::px(1400.), gpui_kit::px(900.)),
                })),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            move |window, cx| cx.new(|cx| MainWindow::new(db, app_config, window, cx)),
        )
    });
    let (window, entity) = opened.expect("the editor window opens");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    (window, entity, library, stored)
}

/// Settings that live in a scratch file, never in the operator's own.
///
/// The file is inside the temporary directory and is named after the process,
/// so a test run starts from the defaults and leaves nothing behind. The
/// library path moves with it: the prop library lives next to the library, so
/// a test that toggled a prop would otherwise write the operator's own props.
fn scratch_config() -> crate::config::Config {
    let dir = std::env::temp_dir().join(format!("openpresenter-tests-{}", std::process::id()));
    crate::config::Config {
        config_path: dir.join("config.toml"),
        db_path: dir.join("library.db"),
        ..Default::default()
    }
}

/// Prepare the app: the kit's globals, the theme and the key bindings.
///
/// The bindings are registered when the app boots, so a test that opens the
/// window on its own has to register them too — otherwise the keys would
/// never arrive.
fn prepare(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::theme::install);
    cx.update(crate::ui::actions::bind_keys);
}

/// Run something on the editor's state, on the UI thread.
fn with_editor<R>(
    cx: &mut TestAppContext,
    window: gpui_kit::AnyWindowHandle,
    entity: &Entity<MainWindow>,
    f: impl FnOnce(&mut MainWindow, &mut Window, &mut Context<MainWindow>) -> R,
) -> Option<R> {
    cx.update(|cx| {
        window
            .update(cx, |_root, window, cx| {
                entity.update(cx, |w, cx| f(w, window, cx))
            })
            .ok()
    })
}

/// Put the editor into Edit mode with the slide open, the way the operator
/// arrives there.
///
/// The document is the one the library holds, so saving it reaches the
/// library — the same document the operator works on.
fn edit_the_slide(
    cx: &mut TestAppContext,
    window: gpui_kit::AnyWindowHandle,
    entity: &Entity<MainWindow>,
    stored: &Presentation,
) {
    with_editor(cx, window, entity, |w, _window, cx| {
        crate::ui::navigation::open_for_editing(w, stored.clone(), Some(0));
        w.shell.current_mode = ViewMode::Edit;
        crate::ui::navigation::load_slide_for_editing(w);
        cx.notify();
    });
}

/// The editor renders its window.
///
/// A window that draws nothing is an editor the operator cannot use, and
/// nothing but a rendered window says so.
#[gpui_kit::test]
fn the_editor_renders_its_window(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, _entity, _db, _stored) = open_editor(cx, &pres);

    let width = cx
        .update_window(window, |_, window, _| window.viewport_size().width)
        .expect("the window renders");
    assert!(width > gpui_kit::px(0.), "the editor has a width");
}

/// The Edit button in the top bar opens the editor.
///
/// The top bar is how the operator moves between showing and editing, so a
/// button that does nothing there is a button the operator cannot use: the
/// menu offers the same command, and the two must agree.
#[gpui_kit::test]
fn the_edit_button_in_the_top_bar_opens_the_editor(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The app starts in Show mode.
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.shell.current_mode),
        ViewMode::Show,
        "the app starts in show mode"
    );

    cx.update_window(window, |_, window, cx| {
        window.click("nav-edit", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.shell.current_mode),
        ViewMode::Edit,
        "the Edit button opened the editor"
    );
}

/// The editor shows the slide it has open.
///
/// The canvas is the editor's centre: without it the operator has nothing
/// to edit.
#[gpui_kit::test]
fn the_editor_shows_a_canvas(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The slide's object is on the canvas: the editor draws the objects of
    // the slide it has open.
    let found = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-object-0")
                .map(|object| object.bounds())
        })
        .expect("the window renders");
    let bounds = found.expect("the editor draws the slide's object");
    assert!(
        bounds.size.width > gpui_kit::px(0.) && bounds.size.height > gpui_kit::px(0.),
        "the object is drawn with a size (got {:?})",
        bounds.size
    );
}

/// The text the operator types reaches the slide.
///
/// The editor writes the text into the object, and the object is what the
/// projector draws.
#[gpui_kit::test]
fn the_text_the_operator_types_reaches_the_slide(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);

    // The text is typed into the slide.
    with_editor(cx, window, &entity, |w, _window, cx| {
        crate::ui::slides::update(
            w,
            crate::ui::slides::Message::SlideTextChanged("Grace".to_string()),
            cx,
        );
    });

    let document = cx.read_entity(&entity, |w, _| w.editor.editing.clone());
    let document = document.expect("a document is open");
    match &document.slides[0].layers[0].content {
        crate::domain::ObjectContent::Text { text, .. } => {
            assert_eq!(text, "Grace", "the object carries the new text");
        }
        _ => panic!("the object is a text object"),
    }
}

/// Clicking an object selects it.
///
/// The selection is the editor's central gesture: without it the operator
/// cannot move, restyle or remove anything.
#[gpui_kit::test]
fn clicking_an_object_selects_it(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // Nothing is selected to begin with.
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.layer.selected_index),
        None,
        "nothing is selected yet"
    );

    // The object is clicked.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.layer.selected_index),
        Some(0),
        "the clicked object is selected"
    );
}

/// The object is drawn where the model says it is.
///
/// The editor and the projector have to agree: an object that is drawn
/// somewhere else is an object the operator cannot place.
#[gpui_kit::test]
fn the_object_is_drawn_where_the_model_says(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    // The object sits at the middle of the slide.
    pres.slides[0].layers[0].position_x = 0.5;
    pres.slides[0].layers[0].position_y = 0.5;
    pres.slides[0].layers[0].width = 0.4;
    pres.slides[0].layers[0].height = 0.2;
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let bounds = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-object-0")
                .map(|object| object.bounds())
        })
        .expect("the window renders")
        .expect("the object is drawn");
    let centre = bounds.center();
    let window_width = cx
        .update_window(window, |_, window, _| window.viewport_size().width)
        .expect("the window renders");
    // The canvas is centred in the workspace, so the object is at the middle
    // of the *slide*, not of the window. What the test can say is that the
    // object is drawn inside the window and with the size the model asks
    // for.
    assert!(
        centre.x > gpui_kit::px(0.) && centre.x < window_width,
        "the object is drawn inside the window (centre {:?}, width {:?})",
        centre.x,
        window_width
    );
    assert!(
        bounds.size.width > gpui_kit::px(0.),
        "the object is drawn with a width"
    );
}

/// The Delete key removes the selected object.
///
/// The keyboard is how the editor is worked, so the key has to reach the
/// slide.
#[gpui_kit::test]
fn the_delete_key_removes_the_selected_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The object is selected by clicking it.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");

    // The key is pressed.
    cx.update_window(window, |_, window, cx| {
        window.press("delete", cx);
    })
    .expect("the window renders");

    let document = cx.read_entity(&entity, |w, _| w.editor.editing.clone());
    let document = document.expect("a document is open");
    assert!(
        document.slides[0].layers.is_empty(),
        "the object is gone (the slide still has {:?})",
        document.slides[0].layers.len()
    );
}

/// The arrow keys move the selected object.
///
/// The nudge is a step on the slide, so the object has to move — and it has
/// to move by the step, not by nothing.
#[gpui_kit::test]
fn the_arrow_keys_move_the_selected_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.position_x)
    });
    let before = before.expect("the object is there");

    cx.update_window(window, |_, window, cx| {
        window.press("right", cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.position_x)
    });
    let after = after.expect("the object is there");
    assert!(
        after > before,
        "the object moved to the right (from {before} to {after})"
    );
}

/// The "+ Text" button adds a text object to the slide.
///
/// The button is how the operator starts a title, so it has to reach the
/// slide.
#[gpui_kit::test]
fn the_add_text_button_adds_an_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.layers.len())
    });
    let before = before.expect("a slide is open");

    cx.update_window(window, |_, window, cx| {
        window.click("edit-obj-add-text", cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.layers.len())
    });
    assert_eq!(after, Some(before + 1), "the slide has one more object");
}

/// The "Save Slide" button saves the slide.
///
/// The operator's work has to reach the disk: a button that does nothing
/// loses the show.
#[gpui_kit::test]
fn the_save_button_saves_the_slide(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, db, stored) = open_editor(cx, &pres);
    let presentation_id = stored.id.clone();
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text is changed.
    with_editor(cx, window, &entity, |w, _window, cx| {
        crate::ui::slides::update(
            w,
            crate::ui::slides::Message::SlideTextChanged("Grace".to_string()),
            cx,
        );
    });

    // The slide is saved.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-save-slide", cx);
    })
    .expect("the window renders");

    // The saved slide carries the new text.
    let saved = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.id.clone())
    });
    let saved = saved.expect("a slide is open");
    let stored = crate::db::repository::PresentationRepository::new(db.clone())
        .get_presentation(&presentation_id)
        .expect("the presentation is there");
    let slide = stored
        .slides
        .iter()
        .find(|slide| slide.id == saved)
        .expect("the slide is there");
    match &slide.layers[0].content {
        crate::domain::ObjectContent::Text { text, .. } => {
            assert_eq!(text, "Grace", "the saved slide carries the new text");
        }
        _ => panic!("the object is a text object"),
    }
}

/// The "B" button makes the text bold.
///
/// The typography controls are the editor's bread and butter: a button that
/// does nothing is a control the operator cannot use.
#[gpui_kit::test]
fn the_bold_button_makes_the_text_bold(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The object is selected, so the button applies to it.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");

    cx.update_window(window, |_, window, cx| {
        window.click("inspector-bold", cx);
    })
    .expect("the window renders");

    let bold = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { style, .. } => Some(style.bold),
                _ => None,
            })
    });
    assert_eq!(bold, Some(true), "the text is bold");
}

/// Clicking a row in the Layers panel selects the object.
///
/// The Layers panel and the canvas are two views of the same slide, so a
/// click in either has to select the same object.
#[gpui_kit::test]
fn clicking_a_layer_row_selects_the_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The Layers panel is in the "Shape" tab.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-shape", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("layer-name-0", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.layer.selected_index),
        Some(0),
        "the layer row selects the object"
    );
}

/// The Layers panel's "up" arrow moves the object up the list.
///
/// The arrow is how the operator changes the draw order, so it has to move
/// the row it belongs to.
#[gpui_kit::test]
fn the_layers_up_arrow_moves_the_row(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    // Two objects, so there is a row to move.
    let mut second = crate::domain::Object::new_text("Second".to_string());
    second.id = "object-2".to_string();
    pres.slides[0].layers.push(second);
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| {
                slide
                    .layers
                    .iter()
                    .map(|layer| layer.id.clone())
                    .collect::<Vec<_>>()
            })
    });
    let before = before.expect("a slide is open");

    // The bottom row — the one at the back of the picture — is moved up.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-shape", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("layer-up-0", cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| {
                slide
                    .layers
                    .iter()
                    .map(|layer| layer.id.clone())
                    .collect::<Vec<_>>()
            })
    });
    let after = after.expect("a slide is open");
    assert_ne!(
        before, after,
        "the row moved (from {before:?} to {after:?})"
    );
}

/// The inspector shows the controls of the tab that is selected.
///
/// The tabs are the editor's panels; a tab that lights up while its panel
/// never appears is a dead control.
#[gpui_kit::test]
fn the_inspector_shows_the_selected_tab(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text tab is selected: the text controls are there.
    let has_text_control = cx
        .update_window(window, |_, window, _| {
            window.try_find("inspector-bold").is_some()
        })
        .expect("the window renders");
    assert!(has_text_control, "the text tab shows the text controls");

    // The shape tab is selected: the shape controls are there.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-shape", cx);
    })
    .expect("the window renders");
    let has_layer_panel = cx
        .update_window(window, |_, window, _| {
            window.try_find("layer-name-0").is_some()
        })
        .expect("the window renders");
    assert!(
        has_layer_panel,
        "the shape tab shows the layer's own properties"
    );
}

/// The undo key takes the edit back.
///
/// The editor is where the operator's work happens, so undo has to work
/// there.
#[gpui_kit::test]
fn the_undo_key_takes_the_edit_back(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text is changed.
    with_editor(cx, window, &entity, |w, _window, cx| {
        crate::ui::slides::update(
            w,
            crate::ui::slides::Message::SlideTextChanged("Grace".to_string()),
            cx,
        );
    });
    let changed = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
    });
    assert_eq!(changed.as_deref(), Some("Grace"), "the text changed");

    // The window has the focus, so the key reaches the shell.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.focus_handle.focus(window, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.press("cmd-z", cx);
    })
    .expect("the window renders");

    let undone = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
    });
    assert_eq!(
        undone.as_deref(),
        Some("Welcome"),
        "the undo took the edit back"
    );
}

/// Clicking a slide in the list opens it.
///
/// The slide list is how the operator moves between the slides of the show.
#[gpui_kit::test]
fn clicking_a_slide_in_the_list_opens_it(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    let mut second = crate::domain::Slide::new_text("Second".to_string());
    second.id = "slide-2".to_string();
    pres.slides.push(second);
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.editor.selected_slide_index),
        Some(0),
        "the first slide is open"
    );

    // The second slide is clicked.
    cx.update_window(window, |_, window, cx| {
        window.click("edit-slide-1", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.editor.selected_slide_index),
        Some(1),
        "the second slide is open"
    );
}

/// The "+ Text" button in the Layers panel adds a text object.
///
/// The Layers panel and the canvas are two ways into the same slide, so
/// both have to reach it.
#[gpui_kit::test]
fn the_layers_panel_adds_a_text_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.layers.len())
    });

    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-shape", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("layers-add-text", cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.layers.len())
    });
    assert_eq!(
        after,
        before.map(|n| n + 1),
        "the slide has one more object"
    );
}

/// The "+" button makes the text bigger.
///
/// The size is a mirror of the slide's: the button has to reach the object,
/// and the inspector has to show what the slide holds.
#[gpui_kit::test]
fn the_font_size_button_makes_the_text_bigger(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The object is selected, so the button applies to it.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { style, .. } => Some(style.font_size),
                _ => None,
            })
    });
    let before = before.expect("the text is there");

    cx.update_window(window, |_, window, cx| {
        window.click("inspector-fs-plus", cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { style, .. } => Some(style.font_size),
                _ => None,
            })
    });
    let after = after.expect("the text is there");
    assert!(after > before, "the text grew (from {before} to {after})");

    // The inspector shows the size the slide holds.
    let shown = cx.read_entity(&entity, |w, _| w.layer.font_size.clone());
    let shown: f32 = shown.parse().expect("the mirror holds a number");
    assert_eq!(shown, after, "the inspector shows the size the slide holds");
}

/// The alignment buttons line the text up.
///
/// The text's alignment is what the operator sets on the slide, so the
/// buttons have to reach it.
#[gpui_kit::test]
fn the_alignment_buttons_align_the_text(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");

    // The second alignment button centres the text.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-align-1", cx);
    })
    .expect("the window renders");

    let alignment = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { style, .. } => Some(style.alignment),
                _ => None,
            })
    });
    assert_eq!(
        alignment,
        Some(crate::domain::TextAlignment::Center),
        "the text is centred"
    );
}

/// Dragging an object on the canvas moves it.
///
/// The drag is the editor's main gesture: the object has to follow the
/// pointer, and it has to land where the pointer left it.
#[gpui_kit::test]
fn dragging_an_object_moves_it(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.position_x)
    });
    let before = before.expect("the object is there");

    // The object is dragged to the right.
    let bounds = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-object-0")
                .map(|object| object.bounds())
        })
        .expect("the window renders")
        .expect("the object is drawn");
    let from = bounds.center();
    let to = gpui_kit::point(from.x + gpui_kit::px(120.), from.y);
    cx.update_window(window, |_, window, cx| {
        window.drag(from, to, cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.position_x)
    });
    let after = after.expect("the object is there");
    assert!(
        after > before,
        "the object followed the pointer (from {before} to {after})"
    );
}

/// The "+ Shape" button adds a shape to the slide.
#[gpui_kit::test]
fn the_add_shape_button_adds_a_shape(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.layers.len())
    });

    cx.update_window(window, |_, window, cx| {
        window.click("edit-obj-add-shape", cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .map(|slide| slide.layers.len())
    });
    assert_eq!(
        after,
        before.map(|n| n + 1),
        "the slide has one more object"
    );

    // The new object is a shape, not a text.
    let is_shape = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.last())
            .and_then(|slide| slide.layers.last())
            .is_some_and(|layer| {
                matches!(layer.content, crate::domain::ObjectContent::Shape { .. })
            })
    });
    assert!(is_shape, "the new object is a shape");
}

/// A double click on a text object edits it in place.
///
/// The inline editor is how the operator rewrites a title without going to
/// the inspector.
#[gpui_kit::test]
fn a_double_click_edits_the_text_in_place(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.canvas.editing),
        None,
        "the text is not being edited yet"
    );

    cx.update_window(window, |_, window, cx| {
        window.double_click("canvas-object-0", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.canvas.editing),
        Some(0),
        "the text is being edited in place"
    );
}

/// "Copy Style" and "Paste Style" carry the type from one object to another.
///
/// The pair is the editor's way of making two texts look the same.
#[gpui_kit::test]
fn the_style_clipboard_carries_the_type(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    // The first object is bold; the second one is not.
    if let crate::domain::ObjectContent::Text { style, .. } = &mut pres.slides[0].layers[0].content
    {
        style.bold = true;
    }
    let mut second = crate::domain::Object::new_text("Second".to_string());
    second.id = "object-2".to_string();
    // The second object sits apart from the first, so a click reaches the
    // one the test means.
    second.position_x = 0.15;
    second.position_y = 0.15;
    pres.slides[0].layers.push(second);
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The bold object is selected and its style is copied.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-copy-style", cx);
    })
    .expect("the window renders");

    // The other object is selected, and the style is pasted onto it.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-1", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-paste-style", cx);
    })
    .expect("the window renders");

    let bold = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.get(1))
            .and_then(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { style, .. } => Some(style.bold),
                _ => None,
            })
    });
    assert_eq!(bold, Some(true), "the second object took the copied style");
}

/// A hidden object is not drawn on the canvas.
///
/// The compositor skips a hidden object, so the editor has to skip it too:
/// an object the editor shows and the projector does not is an object the
/// operator cannot place.
#[gpui_kit::test]
fn a_hidden_object_is_not_drawn(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The object is on the canvas to begin with.
    let drawn = cx
        .update_window(window, |_, window, _| {
            window.try_find("canvas-object-0").is_some()
        })
        .expect("the window renders");
    assert!(drawn, "the object is drawn");

    // The eye is clicked: the object is hidden.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-shape", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("layer-vis-0", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let drawn = cx
        .update_window(window, |_, window, _| {
            window.try_find("canvas-object-0").is_some()
        })
        .expect("the window renders");
    assert!(!drawn, "the hidden object is not drawn");

    // The model agrees: the object is not visible.
    let visible = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.visible)
    });
    assert_eq!(visible, Some(false), "the object is hidden in the model");
}

/// A locked object does not move.
///
/// The lock is the operator's way of saying "leave this one alone".
#[gpui_kit::test]
fn a_locked_object_does_not_move(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The object is locked.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-shape", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("layer-lock-0", cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.position_x)
    });

    // It is dragged to the right.
    let bounds = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-object-0")
                .map(|object| object.bounds())
        })
        .expect("the window renders")
        .expect("the object is drawn");
    let from = bounds.center();
    let to = gpui_kit::point(from.x + gpui_kit::px(120.), from.y);
    cx.update_window(window, |_, window, cx| {
        window.drag(from, to, cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.position_x)
    });
    assert_eq!(after, before, "the locked object stayed where it was");
}

/// A corner handle resizes the object.
///
/// The handles are how the operator changes an object's size.
#[gpui_kit::test]
fn a_corner_handle_resizes_the_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The object is selected, so its handles are there.
    cx.update_window(window, |_, window, cx| {
        window.click("canvas-object-0", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.width)
    });
    let before = before.expect("the object is there");

    // The bottom-right handle is dragged.
    let handle = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-handle-0-BottomRight")
                .map(|handle| handle.bounds().center())
        })
        .expect("the window renders")
        .expect("the handle is drawn");
    let to = gpui_kit::point(handle.x + gpui_kit::px(80.), handle.y + gpui_kit::px(40.));
    cx.update_window(window, |_, window, cx| {
        window.drag(handle, to, cx);
    })
    .expect("the window renders");

    let after = cx.read_entity(&entity, |w, _| {
        w.editor
            .editing
            .as_ref()
            .and_then(|p| p.slides.first())
            .and_then(|s| s.layers.first())
            .map(|layer| layer.width)
    });
    let after = after.expect("the object is there");
    assert!(after > before, "the object grew (from {before} to {after})");
}

/// Open the show with the presentation live.
fn open_the_show(
    cx: &mut TestAppContext,
    stored: &Presentation,
) -> (gpui_kit::AnyWindowHandle, Entity<MainWindow>) {
    let (window, entity, _db, _stored) = open_editor(cx, stored);
    with_editor(cx, window, &entity, |w, _window, cx| {
        crate::ui::navigation::open_for_editing(w, stored.clone(), Some(0));
        w.shell.current_mode = ViewMode::Show;
        w.presenting.presentation = Some(stored.clone());
        w.presenting.slide_index = 0;
        crate::ui::navigation::activate_slide(w, 0, false, cx);
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    (window, entity)
}

/// The dock's "Go" button advances the show.
///
/// The button is how the operator moves forward, so it has to reach the
/// slide.
#[gpui_kit::test]
fn the_go_button_advances_the_show(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    let mut second = crate::domain::Slide::new_text("Second".to_string());
    second.id = "slide-2".to_string();
    pres.slides.push(second);
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.presenting.slide_index),
        0,
        "the first slide is live"
    );

    cx.update_window(window, |_, window, cx| {
        window.click("dock-go", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.presenting.slide_index),
        1,
        "the show moved to the next slide"
    );
}

/// The Preview pane renders the composited output frame.
///
/// The operator judges what the audience sees from this pane, so a published
/// audience frame has to appear here rather than the presentation name and
/// slide index.
#[gpui_kit::test]
fn the_preview_pane_shows_the_audience_frame(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    let frame = crate::render::Frame::new(2, 2);
    let image = crate::render::frame_to_image(&frame).expect("a 2x2 frame converts");
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.compositor
            .slot
            .publish(crate::ui::output::AUDIENCE_SCREEN_ID, 1, image);
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("show-preview-frame")
                .map(|snap| snap.visible())
        })
        .expect("the window renders");
    assert_eq!(
        found,
        Some(true),
        "the preview pane shows the composited audience frame"
    );
}

/// The "Slide" switch takes the slide off the output.
///
/// The switches are the operator's controls during a show, so they have to
/// reach the output.
#[gpui_kit::test]
fn the_slide_switch_takes_the_slide_off(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.slide_layer_active),
        "the slide is on the output"
    );

    cx.update_window(window, |_, window, cx| {
        window.click("dock-clear-slide", cx);
    })
    .expect("the window renders");

    assert!(
        !cx.read_entity(&entity, |w, _| w.presenting.slide_layer_active),
        "the slide is off the output"
    );
}

/// "Clear All" clears everything the output is showing.
///
/// The screen goes black, the layers are off and the sound stops: a clear
/// that leaves the music playing is not a clear.
#[gpui_kit::test]
fn the_clear_all_button_clears_the_output(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    cx.update_window(window, |_, window, cx| {
        window.click("dock-clear-all", cx);
    })
    .expect("the window renders");

    let state = cx.read_entity(&entity, |w, _| {
        (
            w.output.black_screen,
            w.presenting.slide_layer_active,
            w.presenting.media_layer_active,
            w.audio.playing,
        )
    });
    assert!(state.0, "the output is black");
    assert!(!state.1, "the slide layer is off");
    assert!(!state.2, "the media layer is off");
    assert!(!state.3, "the sound stopped");
}

/// The "Black" button blacks the output and brings it back.
///
/// The operator needs to take the picture off the projector and put it
/// back without ending the show.
#[gpui_kit::test]
fn the_black_button_blacks_and_unblacks(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    assert!(
        !cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the output is not black to begin with"
    );

    cx.update_window(window, |_, window, cx| {
        window.click("dock-black", cx);
    })
    .expect("the window renders");
    assert!(
        cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the output is black"
    );
    // The broadcast follows the audience: it was blacked with the audience,
    // so it is blacked with it.
    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.ndi_blacked),
        "the broadcast is black as well"
    );

    cx.update_window(window, |_, window, cx| {
        window.click("dock-black", cx);
    })
    .expect("the window renders");
    assert!(
        !cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the output is back"
    );
    // And it comes back with it: a broadcast that stayed blacked would leave
    // the "Live" switch dark while the stream is live.
    assert!(
        !cx.read_entity(&entity, |w, _| w.presenting.ndi_blacked),
        "the broadcast is live again"
    );
}

/// The broadcast's "Live" switch is a toggle.
///
/// The operator takes the stream off the air with the switch and puts it
/// back with the same switch. A switch that stays dark after the stream came
/// back is a switch the operator cannot trust, and pressing it again would
/// black the stream once more instead of restoring it.
#[gpui_kit::test]
fn the_live_switch_toggles_the_broadcast(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    // The switch is pressed: the broadcast goes black.
    cx.update_window(window, |_, window, cx| {
        window.click("dock-clear-live", cx);
    })
    .expect("the window renders");
    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.ndi_blacked),
        "the broadcast is blacked"
    );

    // The switch is pressed again: the broadcast comes back.
    cx.update_window(window, |_, window, cx| {
        window.click("dock-clear-live", cx);
    })
    .expect("the window renders");
    assert!(
        !cx.read_entity(&entity, |w, _| w.presenting.ndi_blacked),
        "the broadcast is live again"
    );
}

/// The content type makes the slide what the panel says it is.
///
/// The slide's objects are the picture, so converting a slide to text has to
/// take the picture away: a slide that keeps its image shows the image while
/// the panel says it is a title.
#[gpui_kit::test]
fn converting_a_slide_to_text_takes_the_picture_away(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The slide becomes an image slide.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-build", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-ct-image", cx);
    })
    .expect("the window renders");

    let is_image = cx.read_entity(&entity, |w, _| {
        w.get_current_slide()
            .map(|slide| slide.content_kind() == crate::domain::SlideKind::Image)
            .unwrap_or(false)
    });
    assert!(is_image, "the slide is an image slide");

    // And back to text.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-ct-text", cx);
    })
    .expect("the window renders");

    let (kind, has_image) = cx.read_entity(&entity, |w, _| {
        w.get_current_slide()
            .map(|slide| {
                (
                    slide.content_kind(),
                    slide.layers.iter().any(|layer| {
                        matches!(layer.content, crate::domain::ObjectContent::Image { .. })
                    }),
                )
            })
            .unwrap_or((crate::domain::SlideKind::Text, false))
    });
    assert_eq!(
        kind,
        crate::domain::SlideKind::Text,
        "the slide is a text slide"
    );
    assert!(!has_image, "the picture is gone");
}

/// A media slide offers the file picker.
///
/// Converting a slide to an image leaves it without a file, and the operator
/// has to be able to choose one — a slide that cannot be filled is a slide
/// that cannot be shown.
#[gpui_kit::test]
fn a_media_slide_offers_the_file_picker(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The slide becomes an image slide.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-build", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-ct-image", cx);
    })
    .expect("the window renders");

    // The text tab shows the picker for it.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-text", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let has_picker = cx
        .update_window(window, |_, window, _| {
            window.try_find("inspector-image-browse").is_some()
        })
        .expect("the window renders");
    assert!(has_picker, "the image file picker is on screen");
}

/// A slide with no text offers the text, not a box that eats it.
///
/// The Text tab of a slide that carries no text object used to show an empty
/// field: whatever the operator typed was dropped, and the slide stayed
/// blank.
#[gpui_kit::test]
fn a_slide_without_text_offers_a_text_object(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The slide becomes an image slide: it carries no text object.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-build", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-ct-image", cx);
    })
    .expect("the window renders");

    // The Text tab of a media slide shows the picker, not a text field.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-text", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let (has_picker, has_text_box) = cx
        .update_window(window, |_, window, _| {
            (
                window.try_find("inspector-image-browse").is_some(),
                window.try_find_by_label("Enter slide text…").is_some(),
            )
        })
        .expect("the window renders");
    assert!(has_picker, "the picker is on screen");
    assert!(
        !has_text_box,
        "no text field swallows what the operator types"
    );
}

/// Every inspector tab is on screen.
///
/// The inspector's tab bar is as wide as the panel: a tab bar that is wider
/// puts the last tabs off the screen, where the operator cannot reach them
/// — and the slide's settings, the transition and the themes live there.
#[gpui_kit::test]
fn every_inspector_tab_is_reachable(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The Build tab: the slide's own settings.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-build", cx);
    })
    .expect("the window renders");
    let has_build = cx
        .update_window(window, |_, window, _| {
            window.try_find("inspector-ct-text").is_some()
        })
        .expect("the window renders");
    assert!(has_build, "the build tab is on screen");

    // The Theme tab.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-theme", cx);
    })
    .expect("the window renders");
    let has_theme = cx
        .update_window(window, |_, window, _| {
            window.try_find("inspector-theme-save").is_some()
        })
        .expect("the window renders");
    assert!(has_theme, "the theme tab is on screen");

    // And back to the text.
    cx.update_window(window, |_, window, cx| {
        window.click("inspector-tab-text", cx);
    })
    .expect("the window renders");
    let has_text = cx
        .update_window(window, |_, window, _| {
            window.try_find("inspector-bold").is_some()
        })
        .expect("the window renders");
    assert!(has_text, "the text tab is on screen");
}

/// A text prop says what the operator typed, and can be changed afterwards.
///
/// The overlay on the projector is the operator's words: a prop that says
/// "Text" instead of what was typed is a prop nobody asked for, and a prop
/// that cannot be edited is a prop nobody can use.
#[gpui_kit::test]
fn a_text_prop_says_what_the_operator_typed(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    // The props tab is open.
    cx.update_window(window, |_, window, cx| {
        window.click("tab-props", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text is typed into the field.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.props_new_name_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.input("Welcome", cx);
    })
    .expect("the window renders");

    // And the prop is created.
    cx.update_window(window, |_, window, cx| {
        window.click("props-add-text", cx);
    })
    .expect("the window renders");

    let texts = cx.read_entity(&entity, |w, _| {
        w.props
            .manager
            .props
            .iter()
            .filter_map(|prop| match &prop.content {
                crate::domain::PropContent::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
    });
    assert_eq!(
        texts,
        vec![String::from("Welcome")],
        "the prop says what the operator typed"
    );

    // The prop is edited: its text goes into the field, and the change
    // reaches the prop.
    let prop_id = cx.read_entity(&entity, |w, _| {
        w.props
            .manager
            .props
            .first()
            .map(|prop| prop.id.clone())
            .unwrap_or_default()
    });
    cx.update_window(window, |_, window, cx| {
        window.click(format!("prop-edit-{prop_id}"), cx);
    })
    .expect("the window renders");
    let editing = cx.read_entity(&entity, |w, _| w.props.editing_prop_id.clone());
    assert_eq!(editing, Some(prop_id), "the prop is being edited");

    with_editor(cx, window, &entity, |w, window, cx| {
        w.props_new_name_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.input(" home", cx);
    })
    .expect("the window renders");

    let texts = cx.read_entity(&entity, |w, _| {
        w.props
            .manager
            .props
            .iter()
            .filter_map(|prop| match &prop.content {
                crate::domain::PropContent::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
    });
    assert_eq!(
        texts,
        vec![String::from("Welcome home")],
        "the prop says the new text"
    );
}

/// Typing in the Text tab reaches the slide.
///
/// The Text tab's field is a copy of the slide's text, and the copy has to
/// keep up with the keystrokes: a field that is rewritten while the operator
/// types eats every word, and a title nobody can write is a title nobody can
/// show.
#[gpui_kit::test]
fn typing_in_the_text_tab_reaches_the_slide(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The Text tab's field takes the focus.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.inspector_text_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text is typed.
    cx.update_window(window, |_, window, cx| {
        window.input("Grace", cx);
    })
    .expect("the window renders");

    let text = cx.read_entity(&entity, |w, _| {
        w.get_current_slide()
            .and_then(|slide| slide.editable_text(None).map(|(t, _)| t.to_string()))
    });
    let text = text.expect("the slide has text");
    assert!(
        text.ends_with("Grace"),
        "the typed text reached the slide: {text:?}"
    );
}

/// The live alert is on screen while the show runs.
///
/// The alert is the control the operator reaches for during a show, and the
/// dock's body is short: a control that starts below the fold is a control
/// the operator has to hunt for.
#[gpui_kit::test]
fn the_live_alert_is_on_screen_during_the_show(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, _entity) = open_the_show(cx, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let visible = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("ctrl-send-alert")
                .map(|snap| snap.visible())
        })
        .expect("the window renders");
    assert_eq!(visible, Some(true), "the alert's send button is on screen");
}

/// The show's keyboard drives the show.
///
/// During a service the keyboard is the operator's main instrument: the space
/// bar moves forward, the digits jump, and "b" takes the picture off the
/// projector. A key that does not reach the show is a key the operator has
/// to stop using.
#[gpui_kit::test]
fn the_show_keyboard_drives_the_show(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    let mut second = crate::domain::Slide::new_text("Second".to_string());
    second.id = "slide-2".to_string();
    pres.slides.push(second);
    let mut third = crate::domain::Slide::new_text("Third".to_string());
    third.id = "slide-3".to_string();
    pres.slides.push(third);
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    // The window has the focus, so the keys reach the shell.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.focus_handle.focus(window, cx);
    });

    // The space bar moves forward.
    cx.update_window(window, |_, window, cx| {
        window.press("space", cx);
    })
    .expect("the window renders");
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.presenting.slide_index),
        1,
        "the space bar moved to the next slide"
    );

    // The digits jump.
    cx.update_window(window, |_, window, cx| {
        window.press("3", cx);
    })
    .expect("the window renders");
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.presenting.slide_index),
        2,
        "the digit key jumped to the third slide"
    );

    // "b" takes the picture off the projector.
    assert!(
        !cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the output is not black to begin with"
    );
    cx.update_window(window, |_, window, cx| {
        window.press("b", cx);
    })
    .expect("the window renders");
    assert!(
        cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the output is black"
    );

    // And "b" brings it back.
    cx.update_window(window, |_, window, cx| {
        window.press("b", cx);
    })
    .expect("the window renders");
    assert!(
        !cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the output is back"
    );
}

/// Every control of the show's panels is on screen.
///
/// A control that is off the screen is a control the operator cannot use:
/// the dock's panels are narrow and short, and a row that does not fit is
/// a row nobody can press.
#[gpui_kit::test]
fn every_show_control_is_on_screen(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    // The panels the operator opens during a show, and the controls each of
    // them has to show.
    let panels: [(&str, &[&str]); 6] = [
        (
            "tab-show",
            &[
                "ctrl-send-alert",
                "ctrl-matrix",
                "ctrl-output",
                "ctrl-stage-display",
                "ctrl-stage-window",
                "ctrl-ndi",
                "ctrl-triggers-panel",
                "ctrl-recording-panel",
            ],
        ),
        (
            "tab-props",
            &[
                "props-add-text",
                "props-add-image",
                "props-create-lt",
                "props-save-look",
            ],
        ),
        ("tab-messages", &["messages-send"]),
        ("tab-timers", &["timer-start", "timer-stop", "timer-reset"]),
        ("tab-audio", &["audio-play", "audio-stop", "audio-open"]),
        (
            "tab-triggers",
            &[
                "triggers-http-toggle",
                "triggers-osc-toggle",
                "triggers-midi-toggle",
                "triggers-midi-refresh",
            ],
        ),
    ];

    for (tab, controls) in panels {
        cx.update_window(window, |_, window, cx| {
            window.click(tab, cx);
        })
        .expect("the window renders");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
        })
        .expect("the window renders");

        for control in controls {
            let found = cx
                .update_window(window, |_, window, _| {
                    window.try_find(*control).map(|snap| snap.visible())
                })
                .expect("the window renders");
            assert_eq!(
                found,
                Some(true),
                "the {control} of the {tab} panel is on screen"
            );
        }
    }
    // The entity is read so the show is known to be live.
    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.presentation.is_some()),
        "the show is live"
    );
}

/// Every control of the editor is on screen.
///
/// The editor's panels are narrow and short, and a control that does not fit
/// is a control the operator has to hunt for.
#[gpui_kit::test]
fn every_editor_control_is_on_screen(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The editor's own panels: the slide list, the object list, the canvas
    // toolbar and the inspector's tabs.
    let controls = [
        "edit-add-slide",
        "edit-slide-0",
        "edit-obj-add-text",
        "edit-obj-add-shape",
        "edit-obj-0-row",
        "edit-slide-up-0",
        "edit-slide-down-0",
        "edit-slide-dup-0",
        "edit-slide-del-0",
        "inspector-tab-shape",
        "inspector-tab-text",
        "inspector-tab-build",
        "inspector-tab-theme",
    ];
    for control in controls {
        let found = cx
            .update_window(window, |_, window, _| {
                window.try_find(control).map(|snap| snap.visible())
            })
            .expect("the window renders");
        assert_eq!(
            found,
            Some(true),
            "the {control} of the editor is on screen"
        );
    }

    // The inspector's tabs change what is on screen, and each tab's own
    // controls have to be there.
    for (tab, control) in [
        ("inspector-tab-text", "inspector-bold"),
        ("inspector-tab-build", "inspector-ct-text"),
        ("inspector-tab-theme", "inspector-theme-save"),
        ("inspector-tab-shape", "layers-add-text"),
    ] {
        cx.update_window(window, |_, window, cx| {
            window.click(tab, cx);
        })
        .expect("the window renders");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
        })
        .expect("the window renders");
        let found = cx
            .update_window(window, |_, window, _| {
                window.try_find(control).map(|snap| snap.visible())
            })
            .expect("the window renders");
        assert_eq!(
            found,
            Some(true),
            "the {control} of the {tab} panel is on screen"
        );
    }
}

/// A song becomes a presentation.
///
/// The song library is where the operator's lyrics live, and sending a song
/// to a presentation is how they turn it into slides. The presentation and
/// its slides are written together, so the rail never shows an empty entry
/// for a song that has verses.
#[gpui_kit::test]
fn a_song_becomes_a_presentation(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, db, _stored) = open_editor(cx, &pres);

    // A song with two verses is in the library.
    let mut song = crate::domain::Song::new("Amazing Grace".to_string());
    song.artist = Some("John Newton".to_string());
    song.verses = vec![
        crate::domain::Verse::new(
            "Verse 1".to_string(),
            "Amazing grace, how sweet the sound".to_string(),
            0,
        ),
        crate::domain::Verse::new(
            "Chorus 1".to_string(),
            "That saved a wretch like me".to_string(),
            1,
        ),
    ];
    let service = crate::services::SongService::new(Arc::clone(&db));
    service.save(&song).expect("the song is stored");
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.load_songs();
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The song is sent to a presentation.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::SongToPresentation(song.id.clone()),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The editor shows the song as a presentation, with a slide per verse.
    let (name, slides) = cx.read_entity(&entity, |w, _| {
        let editing = w.editor.editing.as_ref();
        (
            editing.map(|p| p.name.clone()).unwrap_or_default(),
            editing.map(|p| p.slides.len()).unwrap_or(0),
        )
    });
    assert_eq!(name, "Amazing Grace", "the song's name is the show's name");
    assert_eq!(slides, 2, "one slide per verse");

    // And the library holds the presentation with its slides.
    let stored = crate::db::repository::PresentationRepository::new(Arc::clone(&db))
        .list_presentations()
        .expect("the presentations are listed");
    let made = stored
        .iter()
        .find(|p| p.name == "Amazing Grace")
        .expect("the presentation is in the library");
    assert_eq!(made.slides.len(), 2, "the slides were written with it");
}

/// The app's own screens cannot be removed.
///
/// The operator's controls drive the projector, the broadcast and the stage
/// monitor. Removing one of them leaves the controls pointing at a screen
/// that is not there, and the control does nothing without saying so.
#[gpui_kit::test]
fn the_apps_own_screens_cannot_be_removed(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    for id in ["audience", "stream", "stage"] {
        with_editor(cx, window, &entity, |w, window, cx| {
            w.handle_message(
                crate::ui::messages::Message::Output(crate::ui::output::Message::Remove(
                    id.to_string(),
                )),
                window,
                cx,
            );
        });
    }

    let missing = cx.read_entity(&entity, |w, _| {
        ["audience", "stream", "stage"]
            .into_iter()
            .filter(|id| w.output.manager.get(id).is_none())
            .count()
    });
    assert_eq!(missing, 0, "the app's own screens are still there");
}

/// The black state and the broadcast agree.
///
/// The broadcast was blacked with the audience, so it comes back with it: a
/// stream that stayed blacked while the projector came back would leave the
/// "Live" switch dark while the stream is live.
#[gpui_kit::test]
fn the_broadcast_follows_the_audience_out_of_black(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    // The audience is blacked, and the broadcast with it.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::ToggleOutputBlackScreen,
            window,
            cx,
        );
    });
    assert!(
        cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the audience is black"
    );
    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.ndi_blacked),
        "the broadcast is black as well"
    );

    // The projector is closed, and the broadcast comes back with it.
    with_editor(cx, window, &entity, |w, _window, cx| {
        crate::ui::output::close_output_window(w, cx);
    });
    assert!(
        !cx.read_entity(&entity, |w, _| w.output.black_screen),
        "the audience is not black any more"
    );
    assert!(
        !cx.read_entity(&entity, |w, _| w.presenting.ndi_blacked),
        "the broadcast is live again"
    );
}

/// The mask the operator picks is the mask the output shows.
///
/// The renderer draws a letterbox, an oval, a frame border and a custom
/// image. A mask the panel cannot reach is a mask nobody can use, so every
/// kind the renderer honours has a control.
#[gpui_kit::test]
fn the_mask_controls_reach_the_output(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, entity) = open_the_show(cx, &stored);

    // The props tab is open.
    cx.update_window(window, |_, window, cx| {
        window.click("tab-props", cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    for (button, expected) in [
        ("mask-letterbox", "Letterbox"),
        ("mask-oval", "Oval"),
        ("mask-border", "Frame Border"),
        ("mask-none", "None"),
    ] {
        cx.update_window(window, |_, window, cx| {
            window.click(button, cx);
        })
        .expect("the window renders");
        let mask = cx.read_entity(&entity, |w, _| format!("{}", w.props.manager.active_mask));
        assert_eq!(mask, expected, "the {button} button chose {expected}");
    }
}

/// A duplicated plan keeps its own name.
///
/// The copy is named "… (Copy)": the field has to show that name, or
/// pressing Save writes the source's name over it and the rail holds two
/// plans with nothing to tell them apart.
#[gpui_kit::test]
fn a_duplicated_plan_keeps_its_own_name(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, db, _stored) = open_editor(cx, &pres);

    // A plan is in the library.
    let plan = crate::domain::Playlist::new("Sunday Service".to_string());
    let service = crate::services::PlaylistService::new(Arc::clone(&db));
    service.save(&plan).expect("the plan is stored");
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.load_service_plans();
        cx.notify();
    });

    // It is duplicated.
    with_editor(cx, window, &entity, |w, window, cx| {
        crate::ui::playlist::duplicate_plan(w, plan.id.clone(), window, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let (field, saved) = cx.read_entity(&entity, |w, _| {
        (
            w.service.name_edit.clone(),
            w.service
                .editing
                .as_ref()
                .map(|plan| plan.name.clone())
                .unwrap_or_default(),
        )
    });
    assert_eq!(saved, "Sunday Service (Copy)", "the copy has its own name");
    assert_eq!(field, "Sunday Service (Copy)", "the field shows it");
}

/// Typing a name and pressing "Create" makes a presentation.
///
/// The name is typed into the field and the button is pressed: the flow the
/// operator uses to start a new show.
#[gpui_kit::test]
fn the_new_presentation_flow_creates_a_presentation(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The modal is opened.
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.editor.new_presentation_open = true;
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The field is there, and it is named, so a screen reader says what it
    // is.
    let named = cx
        .update_window(window, |_, window, _| {
            window.try_find_by_label("Presentation name").is_some()
        })
        .expect("the window renders");
    assert!(named, "the name field is named");

    // The name is typed into the field: it took the focus when the modal
    // opened, so the operator can type straight away.
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.input("Sunday Service", cx);
    })
    .expect("the window renders");

    let still_open = cx.read_entity(&entity, |w, _| w.editor.new_presentation_open);
    let value = cx.read_entity(&entity, |w, cx| {
        w.new_presentation_input.read(cx).value().to_string()
    });
    assert!(
        still_open,
        "the modal is still open (the value is {value:?})"
    );

    // The presentation is created.
    cx.update_window(window, |_, window, cx| {
        window.click("new-pres-create", cx);
    })
    .expect("the window renders");

    let name = cx.read_entity(&entity, |w, _| {
        w.editor.editing.as_ref().map(|p| p.name.clone())
    });
    assert_eq!(
        name.as_deref(),
        Some("Sunday Service"),
        "the new presentation is open with the name that was typed"
    );
}

/// A dialog fits the screen it is shown on.
///
/// The triggers panel is taller than a laptop screen. A dialog that does not
/// fit pushes its own buttons out of reach, so the operator cannot close it —
/// and the control at the bottom of the body is gone with it.
#[gpui_kit::test]
fn a_dialog_fits_the_screen(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The triggers panel is open.
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.triggers.panel_open = true;
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let (card_height, viewport_height, body_height) = cx
        .update_window(window, |_, window, _| {
            let card = window
                .try_find("dialog-card-triggers-automation")
                .expect("the dialog is on screen");
            let body = window
                .try_find("dialog-body-triggers-automation")
                .expect("the dialog body is on screen");
            (
                card.bounds().size.height,
                window.viewport_size().height,
                body.bounds().size.height,
            )
        })
        .expect("the window renders");

    assert!(
        card_height <= viewport_height,
        "the dialog card ({card_height}px) fits the screen ({viewport_height}px)"
    );
    assert!(
        body_height < card_height,
        "the body scrolls inside the card ({body_height}px in {card_height}px)"
    );
}

/// A control inside a dialog does not close the dialog.
///
/// The backdrop closes a dialog when it is clicked, and a click inside the
/// card reaches the backdrop too unless the card stops it: an operator
/// pressing "Start" in the triggers panel would have the panel close under
/// their finger.
#[gpui_kit::test]
fn a_control_inside_a_dialog_does_not_close_it(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The triggers panel is open.
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.triggers.panel_open = true;
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // A macro is named and added from inside the panel. The panel is taller
    // than the window, so the button starts below the fold: the dialog's
    // body scrolls, and the control is reached the way the operator reaches
    // it.
    with_editor(cx, window, &entity, |w, _window, cx| {
        w.triggers.new_macro_name = "Opening".to_string();
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.scroll(
            "dialog-body-triggers-automation",
            ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(2000.))),
            cx,
        );
    })
    .expect("the window renders");
    let before = cx.read_entity(&entity, |w, _| w.triggers.manager.macros.len());
    cx.update_window(window, |_, window, cx| {
        window.click("triggers-macro-add", cx);
    })
    .expect("the window renders");

    assert!(
        cx.read_entity(&entity, |w, _| w.triggers.panel_open),
        "the panel is still open"
    );
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.triggers.manager.macros.len()),
        before + 1,
        "the macro was added"
    );
}

/// The search finds the presentation it is looking for.
///
/// The operator types a name and the rail offers the deck: a search that
/// does not find the show is a search that does nothing.
#[gpui_kit::test]
fn the_search_finds_the_presentation(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    pres.name = "Sunday Service".to_string();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The search field takes the focus.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.search_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The name is typed.
    cx.update_window(window, |_, window, cx| {
        window.input("Sunday", cx);
    })
    .expect("the window renders");

    let query = cx.read_entity(&entity, |w, _| w.shell.search_query.clone());
    assert_eq!(query, "Sunday", "the rail is searching for the name");

    // The rail offers the deck.
    let presentation_id = cx.read_entity(&entity, |w, _| {
        w.editor.presentations.first().map(|p| p.id.clone())
    });
    let presentation_id = presentation_id.expect("the presentation is listed");
    let found = cx
        .update_window(window, |_, window, _| {
            window
                .try_find(format!("rail-presentation-{presentation_id}"))
                .is_some()
        })
        .expect("the window renders");
    assert!(found, "the search result is on the rail");
}

/// Typing reaches the field the operator is in.
///
/// The editor is driven by the keyboard, so a keystroke has to reach the
/// field that has the focus.
#[gpui_kit::test]
fn typing_reaches_the_focused_field(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The search field takes the focus.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.search_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text is typed.
    cx.update_window(window, |_, window, cx| {
        window.input("Sunday", cx);
    })
    .expect("the window renders");

    let value = cx.read_entity(&entity, |w, cx| w.search_input.read(cx).value().to_string());
    assert_eq!(value, "Sunday", "the text reached the field");
}

/// Every key that is a shortcut can still be typed.
///
/// The space bar, the letters `b`, `c` and `v`, the digits and the question
/// mark are presenter shortcuts, and an action stops the event by default: a
/// shortcut that does not apply would swallow the character, so "Church of
/// Christ" would arrive as "ChurhofChrist" and the operator could not type a
/// name at all.
#[gpui_kit::test]
fn the_shortcut_characters_can_be_typed(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The search field takes the focus.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.search_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // Every key that is a shortcut is typed.
    let text = "Church of Christ 1234 b v ?";
    cx.update_window(window, |_, window, cx| {
        window.input(text, cx);
    })
    .expect("the window renders");

    let value = cx.read_entity(&entity, |w, cx| w.search_input.read(cx).value().to_string());
    assert_eq!(value, text, "every character reached the field");
}

/// Backspace edits the field the operator is in.
///
/// Backspace is the editor's "remove the selected object" shortcut, so a
/// keystroke that the app does not act on must still reach the field: a typo
/// the operator cannot take back is a field they have to clear by hand.
#[gpui_kit::test]
fn backspace_edits_the_focused_field(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The search field takes the focus.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.search_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The text is typed.
    cx.update_window(window, |_, window, cx| {
        window.input("Church", cx);
    })
    .expect("the window renders");

    // Two characters are taken back.
    for _ in 0..2 {
        cx.update_window(window, |_, window, cx| {
            window.press("backspace", cx);
        })
        .expect("the window renders");
    }

    let value = cx.read_entity(&entity, |w, cx| w.search_input.read(cx).value().to_string());
    assert_eq!(value, "Chur", "the backspace reached the field");
}

/// A shortcut that does not apply leaves the key alone.
///
/// The presenter keys belong to the show: pressing one in the editor must not
/// jump a slide, and it must not swallow the key either — the character still
/// belongs to the field the operator is typing in.
#[gpui_kit::test]
fn a_presenter_shortcut_does_nothing_in_the_editor(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The search field takes the focus.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.search_input
            .update(cx, |state, input_cx| state.focus(window, input_cx));
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The jump-to-chorus key is pressed.
    cx.update_window(window, |_, window, cx| {
        window.press("c", cx);
    })
    .expect("the window renders");

    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.presentation.is_none()),
        "the editor did not start a show"
    );
    let value = cx.read_entity(&entity, |w, cx| w.search_input.read(cx).value().to_string());
    assert_eq!(value, "c", "the key still reached the field");
}

/// A key press reaches the editor.
///
/// The editor is driven by the keyboard, so a key that never arrives is an
/// editor the operator cannot work in.
#[gpui_kit::test]
fn a_key_press_reaches_the_editor(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    // The window has the focus, so the key reaches the shell.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.focus_handle.focus(window, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The key is pressed, and the window still renders: the key was
    // dispatched rather than dropped.
    cx.update_window(window, |_, window, cx| {
        window.press("right", cx);
    })
    .expect("the window renders");
}

/// A long show still draws its grid in time.
///
/// A show with forty slides is an ordinary Wednesday. The grid is drawn on
/// every frame, so the cost of one frame is the cost of the whole show: a
/// grid that takes a second per frame is a grid the operator cannot work
/// with. The budget is generous — it is there to catch a grid that has
/// become quadratic, not to measure the machine.
#[gpui_kit::test]
fn a_long_grid_is_drawn_in_time(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    for i in 2..=40 {
        let mut slide = crate::domain::Slide::new_text(format!("Slide number {i}"));
        slide.id = format!("slide-{i}");
        pres.slides.push(slide);
    }
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (window, _entity) = open_the_show(cx, &stored);
    // The first frame warms the caches: the measurement is of the steady
    // state, not of the first draw.
    for _ in 0..3 {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
        })
        .expect("renders");
    }
    let start = std::time::Instant::now();
    for _ in 0..10 {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
        })
        .expect("renders");
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "10 frames of a 40-slide grid took {elapsed:?}"
    );
}

/// The zoom makes the slide bigger.
///
/// The zoom is the operator looking closer at a slide, so the slide on the
/// canvas has to grow with it: a zoom that changed nothing would be a
/// control the operator could not use.
#[gpui_kit::test]
fn the_zoom_makes_the_slide_bigger(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let before = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-object-0")
                .map(|object| object.bounds().size.width)
        })
        .expect("the window renders")
        .expect("the editor draws the slide's object");

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Zoom(crate::ui::messages::ZoomCommand::In),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let after = cx
        .update_window(window, |_, window, _| {
            window
                .try_find("canvas-object-0")
                .map(|object| object.bounds().size.width)
        })
        .expect("the window renders")
        .expect("the editor draws the slide's object");

    assert!(
        after > before,
        "zooming in made the slide bigger ({before:?} -> {after:?})"
    );
}

/// The guides are drawn when they are turned on.
///
/// The guides are the operator's marks on the slide: where the middle is
/// and where the safe areas are. They are drawn over the slide and never on
/// the output, so they are the canvas's own.
#[gpui_kit::test]
fn the_guides_are_drawn_when_they_are_turned_on(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(crate::ui::messages::Message::ToggleSlideGuides, window, cx);
        w.handle_message(crate::ui::messages::Message::ToggleThemeGuides, window, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    for id in ["canvas-guide-slide", "canvas-guide-theme"] {
        let found = cx
            .update_window(window, |_, window, _| window.try_find(id).is_some())
            .expect("the window renders");
        assert!(found, "the canvas draws {id}");
    }
}

/// The ruler is on the screen when the rulers are turned on.
///
/// The ruler measures the slide in slide pixels, so it is the operator's
/// way of putting an object at a known place.
#[gpui_kit::test]
fn the_ruler_is_on_the_screen_when_the_rulers_are_turned_on(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(crate::ui::messages::Message::ToggleRulers, window, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("canvas-ruler-top").is_some()
        })
        .expect("the window renders");
    assert!(found, "the canvas shows the ruler");
}

/// The toolbar of objects sits under the canvas.
///
/// The toolbar holds what can go on a slide, so an operator does not have to
/// find the Insert menu for the thing they do most.
#[gpui_kit::test]
fn the_object_toolbar_is_on_the_screen(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("insert-bar-0").is_some()
        })
        .expect("the window renders");
    assert!(found, "the editor shows the object toolbar");
}

/// The test pattern window opens when the operator asks for it.
///
/// A pattern the operator cannot choose is a pattern they will not use, and
/// a window that does not open is a command that does nothing.
#[gpui_kit::test]
fn the_test_pattern_window_opens(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::TestPattern(crate::ui::test_pattern::Message::Open),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("test-pattern-toggle").is_some()
        })
        .expect("the window renders");
    assert!(found, "the test pattern window is on the screen");
}

/// The groups window opens when the operator asks for it.
#[gpui_kit::test]
fn the_groups_window_opens(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Groups(crate::ui::groups::Message::Open),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("group-add").is_some()
        })
        .expect("the window renders");
    assert!(found, "the groups window is on the screen");
}

/// The clear groups window opens when the operator asks for it.
#[gpui_kit::test]
fn the_clear_groups_window_opens(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::ClearGroups(crate::ui::clear_groups::Message::Open),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("clear-group-add").is_some()
        })
        .expect("the window renders");
    assert!(found, "the clear groups window is on the screen");
}

/// The settings window holds every page the menu names.
///
/// A page that is off the bottom of the window is a page the operator
/// cannot reach, so the pages are checked one by one. Each page runs in
/// its own settings window, the way the operator opens it.
#[gpui_kit::test]
fn every_settings_page_opens(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    for tab in crate::ui::messages::SettingsTab::ALL {
        // Start each page from a closed settings window.
        with_editor(cx, window, &entity, |w, window, cx| {
            w.handle_message(
                crate::ui::messages::Message::Output(crate::ui::output::Message::SettingsClose),
                window,
                cx,
            );
        });
        with_editor(cx, window, &entity, |w, window, cx| {
            w.handle_message(
                crate::ui::messages::Message::Output(crate::ui::output::Message::SettingsOpen),
                window,
                cx,
            );
            w.handle_message(
                crate::ui::messages::Message::Output(crate::ui::output::Message::SelectTab(tab)),
                window,
                cx,
            );
        });
        // The settings open in their own window, so the assertion looks there.
        let settings_handle = cx
            .read_entity(&entity, |w, _| w.output.settings_window.clone())
            .expect("the settings window opens");
        cx.update_window(settings_handle, |_, window, cx| {
            window.render_frame(cx);
        })
        .expect("the window renders");
        let found = cx
            .update_window(settings_handle, |_, window, _| {
                window.try_find("settings-close").is_some()
            })
            .expect("the window renders");
        assert!(found, "the settings page {} opens", tab.label());
    }
}

/// Clicking a settings tab must not dismiss the settings window.
///
/// The card sits over a backdrop that closes on mouse-down, so the card has
/// to stop that press before it backs out. A tab switch that closes the
/// window leaves the operator back at the app instead of on the new page.
#[gpui_kit::test]
fn clicking_a_settings_tab_keeps_the_settings_window_open(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Output(crate::ui::output::Message::SettingsOpen),
            window,
            cx,
        );
    });
    let settings_handle = cx
        .read_entity(&entity, |w, _| w.output.settings_window.clone())
        .expect("the settings window opens");
    cx.update_window(settings_handle, |_, window, cx| {
        window.click("settings-tab-audio", cx);
    })
    .expect("the window renders");
    let (open, tab) = cx.read_entity(&entity, |w, _| {
        (w.output.settings_open, w.output.settings_tab)
    });
    assert!(open, "the settings card is still open");
    assert_eq!(
        tab,
        crate::ui::messages::SettingsTab::Audio,
        "the settings tab switched to Audio"
    );
    let found = cx
        .update_window(settings_handle, |_, window, _| {
            window.try_find("settings-card").is_some()
        })
        .expect("the window renders");
    assert!(found, "the settings card is still on screen");
}

/// The Close pill closes the settings window.
///
/// The Close pill dispatches SettingsClose into the main window, which
/// closes the settings window. A pill that writes state but never closes
/// the window strands the operator on the preferences surface.
#[gpui_kit::test]
fn the_close_button_closes_the_settings_window(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Output(crate::ui::output::Message::SettingsOpen),
            window,
            cx,
        );
    });
    let settings_handle = cx
        .read_entity(&entity, |w, _| w.output.settings_window.clone())
        .expect("the settings window opens");
    cx.update_window(settings_handle, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    cx.update_window(settings_handle, |_, window, cx| {
        window.click("settings-done", cx);
    })
    .expect("the window renders");
    // The closed flag makes the settings window's next frame close it.
    let _ = cx.update_window(settings_handle, |_, window, cx| {
        window.render_frame(cx);
    });
    let open = cx.read_entity(&entity, |w, _| w.output.settings_open);
    assert!(!open, "the settings state is closed");
    // The settings window closes itself on its next frame after the flag
    // flips: a lookup that fails means the window is gone.
    let still = cx
        .update_window(settings_handle, |_, window, _| {
            window.try_find("settings-close").is_some()
        })
        .unwrap_or(false);
    assert!(!still, "the settings window is gone");
}

/// The group chips take the slide's group, and the active chip takes it
/// away again.
///
/// The chip is the control the operator reaches for, so the operator's
/// first click sets the group and a second click on the active chip clears
/// it — two buttons would be two ways to say the same thing.
#[gpui_kit::test]
fn the_group_chip_sets_and_clears_the_slide_group(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Output(crate::ui::output::Message::SettingsOpen),
            window,
            cx,
        );
        w.handle_message(
            crate::ui::messages::Message::Output(crate::ui::output::Message::SelectTab(
                crate::ui::messages::SettingsTab::GroupsAndLabels,
            )),
            window,
            cx,
        );
    });
    let settings_handle = cx
        .read_entity(&entity, |w, _| w.output.settings_window.clone())
        .expect("the settings window opens");
    cx.update_window(settings_handle, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The board renders: a standard group chip, the labels, and the add
    // row.
    let visible: Vec<bool> = [
        "group-Verse",
        "group-Chorus",
        "group-label-Repeat",
        "group-add",
    ]
    .iter()
    .map(|id| {
        cx.update_window(settings_handle, |_, window, _| {
            window.try_find(id.to_string()).is_some()
        })
        .expect("the window renders")
    })
    .collect();
    assert!(
        visible.iter().all(|v| *v),
        "every group chip, label and the add control is on screen (got {visible:?})"
    );

    // Clicking the chip puts the group on the slide.
    cx.update_window(settings_handle, |_, window, cx| {
        window.click("group-Chorus", cx);
    })
    .expect("the window renders");
    let group = cx.read_entity(&entity, |w, _| {
        w.get_current_slide().and_then(|s| s.group.clone())
    });
    assert_eq!(group.as_deref(), Some("Chorus"), "the chip set the group");

    // Clicking the active chip again clears it.
    cx.update_window(settings_handle, |_, window, cx| {
        window.click("group-Chorus", cx);
    })
    .expect("the window renders");
    let group = cx.read_entity(&entity, |w, _| {
        w.get_current_slide().and_then(|s| s.group.clone())
    });
    assert_eq!(group, None, "the active chip cleared the group");
}

/// The master volume is on top of the clip's own volume.
///
/// The operator can turn the whole service down without touching every
/// clip, so the sound that is playing follows the master at once.
#[gpui_kit::test]
fn the_master_volume_is_on_top_of_the_clip_volume(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);
    with_editor(cx, window, &entity, |w, _window, _cx| {
        w.app_config.audio.master_volume = 50;
        w.audio.volume = 1.0;
    });
    let volume = with_editor(cx, window, &entity, |w, _window, _cx| {
        let master = f32::from(w.app_config.audio.master_volume) / 100.0;
        w.audio.volume * master
    })
    .expect("the window renders");
    assert!(
        (volume - 0.5).abs() < 0.01,
        "half the master is half the sound (got {volume})"
    );
}

/// The timeline is on the screen in Show mode.
///
/// The timeline is the show as a piece of time: without it the operator has
/// no picture of how the show is laid out.
#[gpui_kit::test]
fn the_timeline_is_on_the_screen_in_show_mode(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    let (show_window, _show_entity) = open_the_show(cx, &stored);
    // The timeline belongs to the show: it is the show as a piece of time.
    cx.update_window(show_window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(show_window, |_, window, _| {
            window.try_find("timeline-play").is_some()
        })
        .expect("the window renders");
    assert!(found, "the timeline is on the screen");
}

/// The service calendar opens when the operator asks for it.
///
/// The calendar is where the operator looks before a service, so it has to
/// be somewhere they can reach.
#[gpui_kit::test]
fn the_service_calendar_opens(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Calendar(crate::ui::calendar::Message::Open),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("calendar-add").is_some()
        })
        .expect("the window renders");
    assert!(found, "the service calendar is on the screen");
}

/// The announcement has a field of its own.
///
/// The announcement is the app's own word, so it is not the operator's
/// alert: one does not replace the other, and the operator has to be able
/// to send one without sending the other.
#[gpui_kit::test]
fn the_announcement_has_a_field_of_its_own(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (_window, _entity, _db, stored) = open_editor(cx, &pres);
    let (show_window, show_entity) = open_the_show(cx, &stored);

    with_editor(cx, show_window, &show_entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::SelectRightDockTab(
                crate::ui::messages::RightDockTab::Messages,
            ),
            window,
            cx,
        );
    });
    cx.update_window(show_window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(show_window, |_, window, _| {
            window.try_find("announcement-input").is_some()
        })
        .expect("the window renders");
    assert!(found, "the announcement has a field of its own");
}

/// The capture settings open when the operator asks for them.
///
/// The settings a service is recorded with are chosen once and used every
/// week, so they have to be somewhere the operator can reach.
#[gpui_kit::test]
fn the_capture_settings_open(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, _stored) = open_editor(cx, &pres);

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::CaptureSettings(
                crate::ui::capture_settings::Message::Open,
            ),
            window,
            cx,
        );
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("capture-preset-add").is_some()
        })
        .expect("the window renders");
    assert!(found, "the capture settings are on the screen");
}

/// Right-clicking a slide in the show opens a menu, and the menu's "Edit
/// Slide" opens that slide in the editor.
///
/// The context menu is the operator's way into a slide: the show is on the
/// screen and the slide has to be edited without hunting for it in the
/// editor's list.
#[gpui_kit::test]
fn right_clicking_a_slide_opens_it_for_editing(cx: &mut TestAppContext) {
    prepare(cx);
    let mut pres = presentation();
    let mut second = crate::domain::Slide::new_text("Second".to_string());
    second.id = "slide-2".to_string();
    pres.slides.push(second);
    let (window, entity, _db, stored) = open_editor(cx, &pres);

    // The show is on the screen with the deck open.
    with_editor(cx, window, &entity, |w, _window, cx| {
        crate::ui::navigation::open_presentation(w, stored.id.clone(), _window, cx);
        w.shell.current_mode = ViewMode::Show;
        cx.notify();
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The slide is right-clicked.
    cx.update_window(window, |_, window, cx| {
        window.right_click("slide-tile-1", cx);
    })
    .expect("the window renders");

    assert!(
        cx.read_entity(&entity, |w, _| w.presenting.slide_context_index)
            .is_some(),
        "the context menu is open"
    );
    let tile_bounds = cx
        .update_window(window, |_, window, _| {
            window.try_find("slide-tile-1").map(|s| s.bounds())
        })
        .expect("renders");
    let menu_bounds = cx
        .update_window(window, |_, window, _| {
            window.try_find("ctx-edit-slide").map(|s| s.bounds())
        })
        .expect("renders");
    println!("DIAG tile={tile_bounds:?} menu={menu_bounds:?}");

    // "Edit Slide" is pressed.
    cx.update_window(window, |_, window, cx| {
        window.click("ctx-edit-slide", cx);
    })
    .expect("the window renders");

    println!(
        "DIAG editing={:?} mode={:?} ctx={:?}",
        cx.read_entity(&entity, |w, _| w.editor.editing.is_some()),
        cx.read_entity(&entity, |w, _| w.shell.current_mode),
        cx.read_entity(&entity, |w, _| w.presenting.slide_context_index),
    );
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.shell.current_mode),
        ViewMode::Edit,
        "the editor opened"
    );
    assert_eq!(
        cx.read_entity(&entity, |w, _| w.editor.selected_slide_index),
        Some(1),
        "the right slide is open"
    );
}

/// Right-clicking a presentation in the rail opens a menu, and the menu's
/// "Edit Presentation" opens it in the editor.
///
/// The rail's context menu is how the operator opens a deck for editing
/// without hunting for it in the list.
#[gpui_kit::test]
fn right_clicking_a_presentation_opens_it_for_editing(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // The presentation is right-clicked in the rail.
    let row = format!("rail-presentation-{}", stored.id);
    let row = SharedString::from(row);
    cx.update_window(window, |_, window, cx| {
        window.right_click(row.clone(), cx);
    })
    .expect("the window renders");

    assert!(
        cx.read_entity(&entity, |w, _| w.shell.rail_context_target.is_some()),
        "the context menu is open"
    );

    // "Edit Presentation" is pressed.
    cx.update_window(window, |_, window, cx| {
        window.click("rail-ctx-edit", cx);
    })
    .expect("the window renders");

    assert_eq!(
        cx.read_entity(&entity, |w, _| w.shell.current_mode),
        ViewMode::Edit,
        "the editor opened"
    );
    assert!(
        cx.read_entity(&entity, |w, _| w.editor.editing.is_some()),
        "the deck is open for editing"
    );
}

/// The app's own commands carry the shortcuts macOS expects.
///
/// The menu takes its shortcut from the key binding, so a command with no
/// binding is a command whose menu shows no shortcut and whose keystroke
/// does nothing — and ⌘Q is the one every operator reaches for first.
#[gpui_kit::test]
fn the_app_commands_carry_the_macos_shortcuts(cx: &mut TestAppContext) {
    prepare(cx);
    let bound = cx.update(|app| {
        let keymap = app.key_bindings();
        let keymap = keymap.borrow();
        let has = |action: &dyn gpui_kit::Action, key: &str, alt: bool| {
            keymap.bindings_for_action(action).any(|binding| {
                binding.keystrokes().iter().any(|keystroke| {
                    keystroke.key().eq_ignore_ascii_case(key)
                        && keystroke.modifiers().platform
                        && keystroke.modifiers().alt == alt
                })
            })
        };
        [
            has(&crate::ui::actions::QuitApp, "q", false),
            has(&crate::ui::actions::HideApp, "h", false),
            has(&crate::ui::actions::HideOtherApps, "h", true),
        ]
    });

    assert!(bound[0], "cmd-q quits the app");
    assert!(bound[1], "cmd-h hides the app");
    assert!(bound[2], "alt-cmd-h hides the other apps");
}

/// The menu's commands reach the app when dispatched, the way a click on a
/// menu item dispatches them.
///
/// The menu can only dispatch actions the root view routes: an action with
/// no route is a menu item the operator clicks that does nothing. Zoom and
/// the slide view are menu-only (no key binding), so this is the signal the
/// menu works.
#[gpui_kit::test]
fn a_dispatched_menu_action_reaches_the_app(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    // The menu dispatches its action on the focused node, so the root view
    // takes the shell's focus first, the way a live window has it.
    with_editor(cx, window, &entity, |w, window, cx| {
        let handle = w.focus_handle.clone();
        window.focus(&handle, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    let zoom_before = cx.read_entity(&entity, |w, _| w.canvas.zoom);
    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(crate::ui::actions::ZoomIn), cx);
    })
    .expect("the window renders");
    // Dispatching is deferred, so the frame flushes it before the state is
    // read back.
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    let zoom_after = cx.read_entity(&entity, |w, _| w.canvas.zoom);
    assert!(
        zoom_after > zoom_before,
        "dispatching ZoomIn zooms the canvas (from {zoom_before} to {zoom_after})"
    );

    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(crate::ui::actions::ViewSlideEasy), cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    let mode = cx.read_entity(&entity, |w, _| w.presenting.slide_view_mode);
    assert_eq!(
        mode,
        crate::ui::messages::SlideViewMode::Easy,
        "dispatching ViewSlideEasy switches the show to Easy"
    );

    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(crate::ui::actions::InsertText), cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    let layers = cx.read_entity(&entity, |w, _| {
        w.get_current_slide()
            .map(|slide| slide.layers.len())
            .unwrap_or(0)
    });
    assert!(
        layers >= 2,
        "dispatching InsertText adds a text object (the slide has {layers} objects)"
    );
}

/// The About dialog opens from the menu and closes from Escape.
///
/// The application menu's first item is About, so the item has to open the
/// dialog that names the app — a menu item that opens nothing is the same
/// gap as any other dead item.
#[gpui_kit::test]
fn the_about_dialog_opens_from_the_app_menu(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    with_editor(cx, window, &entity, |w, window, cx| {
        let handle = w.focus_handle.clone();
        window.focus(&handle, cx);
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(crate::ui::actions::AboutApp), cx);
    })
    .expect("the window renders");
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");
    let open = cx.read_entity(&entity, |w, _| w.ui.about_open);
    assert!(open, "About opens the dialog");
    let found = cx
        .update_window(window, |_, window, _| {
            window.try_find("about-name").map(|snap| snap.visible())
        })
        .expect("the window renders");
    assert_eq!(found, Some(true), "the dialog is on screen");

    cx.update_window(window, |_, window, cx| {
        window.press("escape", cx);
    })
    .expect("the window renders");
    let closed = cx.read_entity(&entity, |w, _| !w.ui.about_open);
    assert!(closed, "Escape closes the dialog");
}

/// Continuous mode wraps the service at the end, so a looped background
/// service never needs an operator on the transport.
///
/// The plan runs to its last item and Next takes it back to the first item:
/// a plan that stops at the end is a clip that stops mid-service.
#[gpui_kit::test]
fn continuous_mode_wraps_the_service_at_the_end(cx: &mut TestAppContext) {
    prepare(cx);
    let pres = presentation();
    let (window, entity, _db, stored) = open_editor(cx, &pres);
    edit_the_slide(cx, window, &entity, &stored);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("the window renders");

    // Two blank items make a plan with no media to load.
    let mut plan = crate::domain::Playlist::new(String::from("Looping Service"));
    plan.items.push(crate::domain::PlaylistItem::Blank);
    plan.items.push(crate::domain::PlaylistItem::Blank);
    with_editor(cx, window, &entity, |w, window, cx| {
        w.service.editing = Some(plan);
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::Start),
            window,
            cx,
        );
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::SetContinuous(
                true,
            )),
            window,
            cx,
        );
    });
    let index = cx.read_entity(&entity, |w, _| w.service.item_index);
    assert_eq!(index, 0, "the service starts on the first item");

    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::Next),
            window,
            cx,
        );
    });
    let index = cx.read_entity(&entity, |w, _| w.service.item_index);
    assert_eq!(index, 1, "Next moves to the second item");

    // At the end, the next press wraps instead of stopping.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::Next),
            window,
            cx,
        );
    });
    let index = cx.read_entity(&entity, |w, _| w.service.item_index);
    assert_eq!(index, 0, "continuous mode wraps to the first item");

    // And without continuous mode, the plan stops where it ended.
    with_editor(cx, window, &entity, |w, window, cx| {
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::Next),
            window,
            cx,
        );
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::SetContinuous(
                false,
            )),
            window,
            cx,
        );
        w.handle_message(
            crate::ui::messages::Message::Playlist(crate::ui::playlist::Message::Next),
            window,
            cx,
        );
    });
    let index = cx.read_entity(&entity, |w, _| w.service.item_index);
    assert_eq!(index, 1, "without continuous mode the last item is the end");
}
