use crate::slides::{ImageFit, SlideContent, TextStyle, Transition};
use crate::ui::handlers::video;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::{Point, Task};

pub(crate) fn add_slide(w: &mut MainWindow) -> Task<Message> {
    w.push_undo();
    if let Some(ref pres) = w.editing_presentation {
        let slide = crate::slides::Slide::new_text(String::new());
        let order = pres.slides.len() as i32;
        let pres_id = pres.id.clone();
        match w.repo.add_slide(&pres_id, &slide, order) {
            Ok(_) => {
                if let Ok(updated) = w.repo.get_presentation(&pres_id) {
                    w.selected_slide_index = Some(updated.slides.len() - 1);
                    w.editing_presentation = Some(updated);
                    w.load_slide_for_editing();
                }
            }
            Err(e) => eprintln!("Failed to add slide: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn add_slide_after(w: &mut MainWindow, after_index: usize) -> Task<Message> {
    w.push_undo();
    if let Some(ref pres) = w.editing_presentation {
        let group = pres
            .slides
            .get(after_index)
            .and_then(|s| s.group_label.clone());
        let mut slide = crate::slides::Slide::new_text(String::new());
        slide.group_label = group;
        let insert_order = (after_index + 1) as i32;
        let pres_id = pres.id.clone();
        match w.repo.add_slide(&pres_id, &slide, insert_order) {
            Ok(_) => {
                if let Ok(updated) = w.repo.get_presentation(&pres_id) {
                    let mut ids: Vec<String> =
                        updated.slides.iter().map(|s| s.id.clone()).collect();
                    if let Some(new_pos) = ids.iter().position(|id| id == &slide.id)
                        && new_pos != after_index + 1
                    {
                        let moved = ids.remove(new_pos);
                        let target = (after_index + 1).min(ids.len());
                        ids.insert(target, moved);
                        let _ = w.repo.reorder_slides(&pres_id, &ids);
                    }
                    if let Ok(reloaded) = w.repo.get_presentation(&pres_id) {
                        let new_index = (after_index + 1).min(reloaded.slides.len() - 1);
                        w.selected_slide_index = Some(new_index);
                        w.editing_presentation = Some(reloaded);
                        w.load_slide_for_editing();
                    }
                }
            }
            Err(e) => eprintln!("Failed to add slide after: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn duplicate_slide(w: &mut MainWindow, index: usize) -> Task<Message> {
    w.push_undo();
    if let Some(ref pres) = w.editing_presentation {
        let pres_id = pres.id.clone();
        if let Some(source) = pres.slides.get(index) {
            let mut dup = source.clone();
            dup.id = uuid::Uuid::new_v4().to_string();
            let order = (index + 1) as i32;
            match w.repo.add_slide(&pres_id, &dup, order) {
                Ok(_) => {
                    if let Ok(updated) = w.repo.get_presentation(&pres_id) {
                        let mut ids: Vec<String> =
                            updated.slides.iter().map(|s| s.id.clone()).collect();
                        if let Some(new_pos) = ids.iter().position(|id| id == &dup.id) {
                            let target = (index + 1).min(ids.len() - 1);
                            if new_pos != target {
                                let moved = ids.remove(new_pos);
                                ids.insert(target, moved);
                                let _ = w.repo.reorder_slides(&pres_id, &ids);
                            }
                        }
                        if let Ok(reloaded) = w.repo.get_presentation(&pres_id) {
                            w.selected_slide_index =
                                Some((index + 1).min(reloaded.slides.len() - 1));
                            w.editing_presentation = Some(reloaded);
                            w.load_slide_for_editing();
                        }
                    }
                }
                Err(e) => eprintln!("Failed to duplicate slide: {e}"),
            }
        }
    }
    Task::none()
}

pub(crate) fn select_slide(w: &mut MainWindow, index: usize) -> Task<Message> {
    w.selected_slide_index = Some(index);
    w.load_slide_for_editing();
    iced::advanced::widget::operate(iced::advanced::widget::operation::scrollable::scroll_to(
        crate::ui::editor::slide_list::scrollable_id(),
        iced::advanced::widget::operation::scrollable::AbsoluteOffset {
            x: Some(0.0_f32),
            y: Some((index / 2) as f32 * 110.0),
        },
    ))
}

pub(crate) fn delete_slide(w: &mut MainWindow, slide_id: String) -> Task<Message> {
    w.push_undo();
    if let Some(ref pres) = w.editing_presentation {
        let pres_id = pres.id.clone();
        match w.repo.delete_slide(&slide_id) {
            Ok(_) => {
                if let Ok(updated) = w.repo.get_presentation(&pres_id) {
                    if let Some(cur) = w.selected_slide_index {
                        if updated.slides.is_empty() {
                            w.selected_slide_index = None;
                        } else if cur >= updated.slides.len() {
                            w.selected_slide_index = Some(updated.slides.len() - 1);
                        }
                    }
                    w.editing_presentation = Some(updated);
                    w.load_slide_for_editing();
                }
            }
            Err(e) => eprintln!("Failed to delete slide: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn move_slide_up(w: &mut MainWindow, index: usize) -> Task<Message> {
    w.push_undo();
    if index > 0
        && let Some(ref mut pres) = w.editing_presentation
    {
        pres.slides.swap(index, index - 1);
        let ids: Vec<String> = pres.slides.iter().map(|s| s.id.clone()).collect();
        let pres_id = pres.id.clone();
        if let Err(e) = w.repo.reorder_slides(&pres_id, &ids) {
            eprintln!("Failed to reorder: {e}");
        } else {
            w.selected_slide_index = Some(index - 1);
        }
    }
    Task::none()
}

pub(crate) fn move_slide_down(w: &mut MainWindow, index: usize) -> Task<Message> {
    w.push_undo();
    let can_move = w
        .editing_presentation
        .as_ref()
        .map(|p| index < p.slides.len() - 1)
        .unwrap_or(false);
    if can_move && let Some(ref mut pres) = w.editing_presentation {
        pres.slides.swap(index, index + 1);
        let ids: Vec<String> = pres.slides.iter().map(|s| s.id.clone()).collect();
        let pres_id = pres.id.clone();
        if let Err(e) = w.repo.reorder_slides(&pres_id, &ids) {
            eprintln!("Failed to reorder: {e}");
        } else {
            w.selected_slide_index = Some(index + 1);
        }
    }
    Task::none()
}

pub(crate) fn group_label_changed(w: &mut MainWindow, label: String) -> Task<Message> {
    w.editing_group_label = label;
    Task::none()
}

pub(crate) fn set_slide_group_label(
    w: &mut MainWindow,
    index: usize,
    label: String,
) -> Task<Message> {
    let label_opt = if label.trim().is_empty() {
        None
    } else {
        Some(label)
    };
    if let Some(ref mut pres) = w.editing_presentation
        && let Some(slide) = pres.slides.get_mut(index)
    {
        slide.group_label = label_opt;
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("Failed to save group label: {e}");
        }
    }
    Task::none()
}

pub(crate) fn slide_text_changed(w: &mut MainWindow, t: String) -> Task<Message> {
    w.editing_slide_text = t;
    Task::none()
}

pub(crate) fn slide_font_size_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.editing_slide_font_size = s;
    Task::none()
}

pub(crate) fn slide_alignment_changed(
    w: &mut MainWindow,
    a: crate::slides::TextAlignment,
) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.alignment = a;
    }
    Task::none()
}

pub(crate) fn slide_color_changed(w: &mut MainWindow, c: crate::slides::Color) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.color = c;
    }
    Task::none()
}

pub(crate) fn slide_shadow_toggled(w: &mut MainWindow, v: bool) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.shadow = v;
    }
    Task::none()
}

pub(crate) fn slide_outline_toggled(w: &mut MainWindow, v: bool) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.outline = v;
    }
    Task::none()
}

pub(crate) fn slide_bold_toggled(w: &mut MainWindow, v: bool) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.bold = v;
    }
    Task::none()
}

pub(crate) fn slide_italic_toggled(w: &mut MainWindow, v: bool) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.italic = v;
    }
    Task::none()
}

pub(crate) fn slide_position_preset(w: &mut MainWindow, x: f32, y: f32) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.position_x = x;
        style.position_y = y;
    }
    Task::none()
}

pub(crate) fn slide_background_changed(
    w: &mut MainWindow,
    bg: crate::slides::Background,
) -> Task<Message> {
    if let Some(s) = w.get_current_slide_mut() {
        s.background = bg;
    }
    Task::none()
}

pub(crate) fn slide_transition_changed(w: &mut MainWindow, t: Transition) -> Task<Message> {
    let dur_str = match &t {
        Transition::Cut => String::from("500"),
        Transition::Fade { duration_ms }
        | Transition::Dissolve { duration_ms }
        | Transition::Slide { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Wipe { duration_ms, .. } => duration_ms.to_string(),
    };
    w.editing_transition_duration = dur_str;
    if let Some(s) = w.get_current_slide_mut() {
        s.transition = t;
    }
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("Failed to save transition: {e}");
        }
    }
    Task::none()
}

pub(crate) fn transition_duration_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.editing_transition_duration = s.clone();
    if let Ok(ms) = s.parse::<u64>()
        && let Some(slide) = w.get_current_slide_mut()
    {
        slide.transition = match slide.transition {
            Transition::Cut => Transition::Cut,
            Transition::Fade { .. } => Transition::Fade { duration_ms: ms },
            Transition::Slide { .. } => Transition::Slide { duration_ms: ms },
            Transition::Dissolve { .. } => Transition::Dissolve { duration_ms: ms },
            Transition::Push { direction, .. } => Transition::Push {
                duration_ms: ms,
                direction,
            },
            Transition::Zoom { .. } => Transition::Zoom { duration_ms: ms },
            Transition::Flip { .. } => Transition::Flip { duration_ms: ms },
            Transition::Clock { .. } => Transition::Clock { duration_ms: ms },
            Transition::Wipe { angle_deg, .. } => Transition::Wipe {
                duration_ms: ms,
                angle_deg,
            },
        };
    }
    Task::none()
}

pub(crate) fn save_slide(w: &mut MainWindow) -> Task<Message> {
    w.push_undo();
    let text = w.editing_slide_text.clone();
    let fs_str = w.editing_slide_font_size.clone();
    if let Some(slide) = w.get_current_slide_mut() {
        if let SlideContent::Text {
            text: ref mut slide_text,
            ref mut style,
        } = slide.content
        {
            *slide_text = text;
            if let Ok(size) = fs_str.parse::<f32>() {
                style.font_size = size;
            }
        }
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("Failed to save slide: {e}");
        }
    }
    Task::none()
}

pub(crate) fn text_dragged(w: &mut MainWindow, point: Point) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.position_x = point.x;
        style.position_y = point.y;
    }
    Task::none()
}

pub(crate) fn text_drag_ended(w: &mut MainWindow) -> Task<Message> {
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("Failed to save text position: {e}");
        }
    }
    Task::none()
}

pub(crate) fn slide_notes_changed(w: &mut MainWindow, notes: String) -> Task<Message> {
    w.editing_slide_notes = notes.clone();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.notes = if notes.trim().is_empty() {
            None
        } else {
            Some(notes)
        };
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save notes: {e}");
        }
    }
    Task::none()
}

pub(crate) fn convert_to_text(w: &mut MainWindow) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && !matches!(slide.content, SlideContent::Text { .. })
    {
        slide.content = SlideContent::Text {
            text: String::new(),
            style: TextStyle::default(),
        };
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save: {e}");
        }
    }
    Task::none()
}

pub(crate) fn convert_to_image(w: &mut MainWindow) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && !matches!(slide.content, SlideContent::Image { .. })
    {
        slide.content = SlideContent::Image {
            path: String::new(),
            fit: ImageFit::default(),
        };
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save: {e}");
        }
    }
    Task::none()
}

pub(crate) fn convert_to_video(w: &mut MainWindow) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && !matches!(slide.content, SlideContent::Video { .. })
    {
        slide.content = SlideContent::Video {
            path: String::new(),
            thumbnail: None,
        };
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save: {e}");
        }
    }
    Task::none()
}

pub(crate) fn pick_image_file(w: &mut MainWindow) -> Task<Message> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
        .set_title("Select Background Image")
        .pick_file()
        && let Some(slide) = w.get_current_slide_mut()
    {
        let fit = match &slide.content {
            SlideContent::Image { fit, .. } => *fit,
            _ => ImageFit::default(),
        };
        slide.content = SlideContent::Image {
            path: path.to_string_lossy().into_owned(),
            fit,
        };
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save: {e}");
        }
    }
    Task::none()
}

pub(crate) fn slide_image_fit_changed(w: &mut MainWindow, fit: ImageFit) -> Task<Message> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Image { fit: ref mut f, .. } = slide.content
    {
        *f = fit;
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save: {e}");
        }
    }
    Task::none()
}

pub(crate) fn pick_video_file(w: &mut MainWindow) -> Task<Message> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Videos", &["mp4", "mov", "avi", "mkv", "webm"])
        .set_title("Select Video")
        .pick_file()
        && let Some(slide) = w.get_current_slide_mut()
    {
        let path_str = path.to_string_lossy().into_owned();
        let thumbnail = crate::media::extract_thumbnail(&path_str, 1.0);
        slide.content = SlideContent::Video {
            path: path_str.clone(),
            thumbnail,
        };
        let c = slide.clone();
        if let Err(e) = w.repo.update_slide(&c) {
            eprintln!("save: {e}");
        }
        video::open_video(w, &path_str);
    }
    Task::none()
}
