use crate::slides::{Slide, Transition};
use crate::ui::handlers::video;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::presenter::TransitionState;
use iced::Task;
use std::time::Instant;

fn begin_slide_change(w: &mut MainWindow, from_slide: Slide, to_idx: usize) {
    let (transition, ndi_slide) = {
        let Some(ref pres) = w.presenting_presentation else {
            return;
        };
        let Some(target) = pres.slides.get(to_idx) else {
            return;
        };
        (target.transition, target.clone())
    };

    w.presenting_slide_index = to_idx;

    if !matches!(transition, Transition::Cut) && !w.reduce_motion {
        w.presenting_transition = Some(TransitionState {
            from_slide,
            transition,
            progress: 0.0,
            start: Instant::now(),
        });
    } else {
        w.presenting_transition = None;
    }

    if let Some(ref ndi) = w.ndi_output {
        ndi.set_slide(ndi_slide);
    }

    video::on_presenter_slide_changed(w, to_idx);
}

pub(crate) fn select_slide(w: &mut MainWindow, i: usize) -> Task<Message> {
    if i == w.presenting_slide_index {
        return Task::none();
    }
    let from_slide = {
        let Some(ref pres) = w.presenting_presentation else {
            return Task::none();
        };
        if pres.slides.get(i).is_none() {
            return Task::none();
        }
        pres.slides[w.presenting_slide_index].clone()
    };
    begin_slide_change(w, from_slide, i);
    Task::none()
}

pub(crate) fn next_slide(w: &mut MainWindow) -> Task<Message> {
    let (from_slide, next) = {
        let Some(ref pres) = w.presenting_presentation else {
            return Task::none();
        };
        let current_i = w.presenting_slide_index;
        let next = (current_i + 1).min(pres.slides.len().saturating_sub(1));
        if next == current_i {
            return Task::none();
        }
        (pres.slides[current_i].clone(), next)
    };
    begin_slide_change(w, from_slide, next);
    Task::none()
}

pub(crate) fn prev_slide(w: &mut MainWindow) -> Task<Message> {
    let (from_slide, prev) = {
        let Some(ref pres) = w.presenting_presentation else {
            return Task::none();
        };
        let current_i = w.presenting_slide_index;
        let prev = current_i.saturating_sub(1);
        if prev == current_i {
            return Task::none();
        }
        (pres.slides[current_i].clone(), prev)
    };
    begin_slide_change(w, from_slide, prev);
    Task::none()
}

pub(crate) fn animation_tick(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref mut ts) = w.presenting_transition {
        let duration_ms = match ts.transition {
            Transition::Fade { duration_ms }
            | Transition::Dissolve { duration_ms }
            | Transition::Slide { duration_ms }
            | Transition::Push { duration_ms, .. }
            | Transition::Zoom { duration_ms }
            | Transition::Flip { duration_ms }
            | Transition::Clock { duration_ms }
            | Transition::Wipe { duration_ms, .. } => duration_ms as f32,
            Transition::Cut => 0.0,
        };
        ts.progress = if duration_ms > 0.0 {
            (ts.start.elapsed().as_millis() as f32 / duration_ms).min(1.0)
        } else {
            1.0
        };
        if ts.progress >= 1.0 {
            w.presenting_transition = None;
        }
    }
    Task::none()
}
