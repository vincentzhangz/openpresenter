use crate::slides::{ServiceItem, ServicePlan};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, ViewMode};
use iced::{Size, Task, window};

pub(crate) fn new_plan(w: &mut MainWindow) -> Task<Message> {
    let plan = ServicePlan::new("New Service Plan".to_string());
    w.service.name_edit = plan.name.clone();
    w.service.editing = Some(plan);
    w.service.to_delete = None;
    Task::none()
}

pub(crate) fn open_plan(w: &mut MainWindow, id: String) -> Task<Message> {
    match w.service.repo.get_plan(&id) {
        Ok(plan) => {
            w.service.name_edit = plan.name.clone();
            w.service.to_delete = None;
            w.service.editing = Some(plan);
        }
        Err(e) => eprintln!("open plan error: {e}"),
    }
    Task::none()
}

pub(crate) fn save_plan(w: &mut MainWindow) -> Task<Message> {
    let Some(mut plan) = w.service.editing.clone() else {
        return Task::none();
    };
    plan.name = w.service.name_edit.clone();
    plan.updated_at = chrono::Utc::now();
    if let Err(e) = w.service.repo.save_plan(&plan) {
        eprintln!("save plan error: {e}");
        return Task::none();
    }
    if let Err(e) = w.service.repo.save_items(&plan.id, &plan.items) {
        eprintln!("save items error: {e}");
        return Task::none();
    }
    w.service.editing = Some(plan);
    w.load_service_plans();
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(existing) = w.delete_confirm_window_id {
        return window::gain_focus(existing);
    }
    w.service.to_delete = Some(id);
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(420.0, 260.0),
        resizable: false,
        ..Default::default()
    });
    w.delete_confirm_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<Message> {
    let Some(id) = w.service.to_delete.take() else {
        return Task::none();
    };
    if let Err(e) = w.service.repo.delete_plan(&id) {
        eprintln!("delete plan error: {e}");
        return Task::none();
    }
    if w.service.editing.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
        w.service.editing = None;
        w.service.name_edit = String::new();
    }
    if w.service.active.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
        w.service.active = None;
    }
    w.load_service_plans();
    if let Some(win_id) = w.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<Message> {
    w.service.to_delete = None;
    if let Some(win_id) = w.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn plan_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.service.name_edit = name;
    Task::none()
}

pub(crate) fn add_presentation_item(w: &mut MainWindow, id: String) -> Task<Message> {
    let name = w
        .presentations
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(ServiceItem::Presentation { id, name });
    }
    Task::none()
}

pub(crate) fn add_song_item(w: &mut MainWindow, id: String) -> Task<Message> {
    let title = w
        .song
        .songs
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.title.clone())
        .unwrap_or_default();
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(ServiceItem::Song { id, title });
    }
    Task::none()
}

pub(crate) fn add_header(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(ServiceItem::Header {
            text: String::new(),
        });
    }
    Task::none()
}

pub(crate) fn add_blank(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(ServiceItem::Blank);
    }
    Task::none()
}

pub(crate) fn item_header_changed(w: &mut MainWindow, index: usize, text: String) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing
        && let Some(ServiceItem::Header { text: t }) = plan.items.get_mut(index)
    {
        *t = text;
    }
    Task::none()
}

pub(crate) fn remove_item(w: &mut MainWindow, index: usize) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing
        && index < plan.items.len()
    {
        plan.items.remove(index);
    }
    Task::none()
}

pub(crate) fn move_item_up(w: &mut MainWindow, index: usize) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing
        && index > 0
        && index < plan.items.len()
    {
        plan.items.swap(index - 1, index);
    }
    Task::none()
}

pub(crate) fn move_item_down(w: &mut MainWindow, index: usize) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing
        && index + 1 < plan.items.len()
    {
        plan.items.swap(index, index + 1);
    }
    Task::none()
}

pub(crate) fn duplicate_item(w: &mut MainWindow, index: usize) -> Task<Message> {
    if let Some(ref mut plan) = w.service.editing
        && index < plan.items.len()
    {
        let item = plan.items[index].clone();
        plan.items.insert(index + 1, item);
    }
    Task::none()
}

pub(crate) fn start_service(w: &mut MainWindow) -> Task<Message> {
    let Some(plan) = w.service.editing.clone() else {
        return Task::none();
    };
    w.service.active = Some(plan.clone());
    w.service.item_index = 0;
    load_item(w, 0);
    Task::none()
}

pub(crate) fn service_next(w: &mut MainWindow) -> Task<Message> {
    let cap = w
        .service
        .active
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    let next = (w.service.item_index + 1).min(cap.saturating_sub(1));
    w.service.item_index = next;
    load_item(w, next);
    Task::none()
}

pub(crate) fn service_prev(w: &mut MainWindow) -> Task<Message> {
    let prev = w.service.item_index.saturating_sub(1);
    w.service.item_index = prev;
    load_item(w, prev);
    Task::none()
}

pub(crate) fn jump_to_item(w: &mut MainWindow, index: usize) -> Task<Message> {
    let cap = w
        .service
        .active
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    let idx = index.min(cap.saturating_sub(1));
    w.service.item_index = idx;
    load_item(w, idx);
    Task::none()
}

pub(crate) fn end_service(w: &mut MainWindow) -> Task<Message> {
    w.service.active = None;
    w.service.item_index = 0;
    Task::none()
}

fn load_item(w: &mut MainWindow, index: usize) {
    let item = w
        .service
        .active
        .as_ref()
        .and_then(|p| p.items.get(index))
        .cloned();

    match item {
        Some(ServiceItem::Presentation { id, .. }) => match w.repo.get_presentation(&id) {
            Ok(pres) => {
                w.presenting_slide_index = 0;
                if let Some(slide) = pres.slides.first()
                    && let Some(ref ndi) = w.ndi_output
                {
                    ndi.set_slide(slide.clone());
                }
                w.presenting_presentation = Some(pres);
                w.current_mode = ViewMode::Show;
            }
            Err(e) => eprintln!("load presentation item error: {e}"),
        },
        Some(ServiceItem::Song { id, .. }) => match w.song.repo.get_song(&id) {
            Ok(song) => w.load_song_for_editing(song),
            Err(e) => eprintln!("load song item error: {e}"),
        },
        Some(ServiceItem::Blank) => {
            if let Some(ref ndi) = w.ndi_output {
                ndi.black_screen();
            }
        }
        _ => {}
    }
}
