use crate::slides::props::{Mask, Prop, PropContent};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub fn prop_toggle(w: &mut MainWindow, id: String) -> Task<Message> {
    w.prop_manager.toggle_prop(&id);
    Task::none()
}

pub fn prop_remove(w: &mut MainWindow, id: String) -> Task<Message> {
    w.prop_manager.remove_prop(&id);
    Task::none()
}

pub fn prop_add_text(w: &mut MainWindow) -> Task<Message> {
    let name = if w.new_prop_name.trim().is_empty() {
        "New Text Prop".to_string()
    } else {
        w.new_prop_name.trim().to_string()
    };
    let prop = Prop {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        content: PropContent::Text {
            text: "Text".to_string(),
            font_size: 32.0,
            color: [1.0, 1.0, 1.0, 1.0],
            bold: false,
            italic: false,
        },
        x: 0.1,
        y: 0.1,
        width: 0.8,
        height: 0.1,
        visible: true,
    };
    w.prop_manager.add_prop(prop);
    w.new_prop_name.clear();
    Task::none()
}

pub fn prop_add_image(w: &mut MainWindow) -> Task<Message> {
    let path = rfd::FileDialog::new()
        .set_title("Select Image for Prop")
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
        .pick_file();

    if let Some(p) = path {
        let filename = p
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Image")
            .to_string();
        let name = if w.new_prop_name.trim().is_empty() {
            filename
        } else {
            w.new_prop_name.trim().to_string()
        };
        let prop = Prop {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            content: PropContent::Image {
                path: p.to_string_lossy().into_owned(),
            },
            x: 0.80,
            y: 0.03,
            width: 0.15,
            height: 0.10,
            visible: true,
        };
        w.prop_manager.add_prop(prop);
        w.new_prop_name.clear();
    }
    Task::none()
}

pub fn new_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.new_prop_name = name;
    Task::none()
}

pub fn lower_third_title_changed(w: &mut MainWindow, title: String) -> Task<Message> {
    w.lower_third_title = title;
    Task::none()
}

pub fn lower_third_subtitle_changed(w: &mut MainWindow, subtitle: String) -> Task<Message> {
    w.lower_third_subtitle = subtitle;
    Task::none()
}

pub fn create_lower_third(w: &mut MainWindow) -> Task<Message> {
    let props =
        crate::slides::props::Prop::new_lower_third(&w.lower_third_title, &w.lower_third_subtitle);
    for prop in props {
        w.prop_manager.add_prop(prop);
    }
    Task::none()
}

pub fn apply_look(w: &mut MainWindow, id: String) -> Task<Message> {
    w.prop_manager.apply_look(&id);
    Task::none()
}

pub fn remove_look(w: &mut MainWindow, id: String) -> Task<Message> {
    w.prop_manager.remove_look(&id);
    Task::none()
}

pub fn save_look(w: &mut MainWindow) -> Task<Message> {
    let name = if w.new_look_name.trim().is_empty() {
        format!("Look {}", w.prop_manager.looks.len() + 1)
    } else {
        w.new_look_name.trim().to_string()
    };
    w.prop_manager.save_look(name);
    w.new_look_name.clear();
    Task::none()
}

pub fn look_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.new_look_name = name;
    Task::none()
}

pub fn set_mask(w: &mut MainWindow, mask: Mask) -> Task<Message> {
    w.prop_manager.active_mask = mask;
    Task::none()
}

pub fn toggle_panel(w: &mut MainWindow) -> Task<Message> {
    w.props_panel_open = !w.props_panel_open;
    Task::none()
}
