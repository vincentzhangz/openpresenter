use crate::slides::{Presentation, Slide, Transition};
use crate::ui::components::group_label_widget;
use crate::ui::messages::Message;
use crate::ui::presenter::TransitionState;
use crate::ui::presenter::canvas::{next_slide_canvas_panel, presenter_canvas_panel};
use crate::ui::theme;
use iced_font_awesome::fa_icon_solid;
use iced::{
    Alignment, Background, Border, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text},
};

pub fn stage_view<'a>(
    presentation: &'a Presentation,
    slide_index: usize,
    transition: Option<&'a TransitionState>,
    clock_secs: u64,
    timer_secs: u64,
    timer_running: bool,
) -> Element<'a, Message> {
    let top_bar = stage_toolbar(clock_secs, timer_secs, timer_running);

    if presentation.slides.is_empty() {
        return container(column![
            top_bar,
            container(
                text("No slides in this presentation")
                    .size(18)
                    .color(theme::TEXT_MUTED),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let current = &presentation.slides[slide_index];
    let next_slide = presentation.slides.get(slide_index + 1);

    let (from_slide, trans_type, trans_progress) = match transition {
        Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
        None => (None, Transition::Cut, 1.0),
    };

    let current_preview = container(presenter_canvas_panel(
        Some(current),
        from_slide,
        trans_type,
        trans_progress,
        None,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::canvas_bg_style);

    let next_col = next_preview_col(next_slide);

    let top_panels = row![current_preview, next_col]
        .width(Length::Fill)
        .height(Length::FillPortion(3));

    let notes_panel = notes_section(current);

    let body = column![top_panels, notes_panel]
        .width(Length::Fill)
        .height(Length::Fill);

    container(column![top_bar, body])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn stage_toolbar(
    clock_secs: u64,
    timer_secs: u64,
    timer_running: bool,
) -> Element<'static, Message> {
    let hh = clock_secs / 3600;
    let mm = (clock_secs % 3600) / 60;
    let ss = clock_secs % 60;
    let clock_str = format!("{hh:02}:{mm:02}:{ss:02}");

    let th = timer_secs / 3600;
    let tm = (timer_secs % 3600) / 60;
    let ts_val = timer_secs % 60;
    let timer_str = format!("{th:02}:{tm:02}:{ts_val:02}");

    let timer_icon: Element<'_, Message> = if timer_running {
        fa_icon_solid("pause").size(13.0).into()
    } else {
        fa_icon_solid("play").size(13.0).into()
    };

    container(
        row![
            button(
                row![
                    fa_icon_solid("arrow-left").size(12.0).color(theme::TEXT_SECONDARY),
                    text(" Show").size(12).color(theme::TEXT_SECONDARY),
                ]
                .align_y(Alignment::Center),
            )
                .on_press(Message::ToggleStageDisplay)
                .padding([6, 14])
                .style(theme::ghost_button),
            Space::new().width(12),
            text("STAGE DISPLAY").size(13).color(theme::TEXT_MUTED),
            Space::new().width(Length::Fill),
            container(text(clock_str).size(20).color(theme::TEXT_PRIMARY)).padding([4, 12]),
            button(timer_icon)
                .on_press(Message::ToggleTimer)
                .padding([6, 10])
                .style(theme::secondary_button),
            container(text(timer_str).size(16).color(theme::WARNING_AMBER)).padding([4, 8]),
            button(fa_icon_solid("rotate-right").size(13.0))
                .on_press(Message::ResetTimer)
                .padding([6, 10])
                .style(theme::ghost_button),
        ]
        .spacing(6)
        .padding([8, 14])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::toolbar_style)
    .into()
}

fn next_preview_col<'a>(next: Option<&'a Slide>) -> Element<'a, Message> {
    let header = container(text("NEXT").size(11).color(theme::LIVE_GREEN))
        .padding([8, 12])
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_DARKEST)),
            ..Default::default()
        });

    let preview: Element<Message> = match next {
        Some(slide) => container(next_slide_canvas_panel(Some(slide)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::canvas_bg_style)
            .into(),
        None => container(
            text("— End of presentation —")
                .size(13)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into(),
    };

    let group_label: Element<Message> = match next.and_then(|s| s.group_label.as_deref()) {
        Some(lbl) => group_label_widget(lbl),
        None => Space::new().height(0).into(),
    };

    container(
        column![header, preview, group_label]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(300)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn notes_section<'a>(slide: &'a Slide) -> Element<'a, Message> {
    let notes_text = slide.notes.as_deref().unwrap_or("");
    let is_empty = notes_text.is_empty();

    let inner: Element<Message> = if is_empty {
        text("(no notes for this slide)")
            .size(14)
            .color(theme::TEXT_MUTED)
            .into()
    } else {
        scrollable(
            text(notes_text)
                .size(22)
                .color(theme::TEXT_PRIMARY)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    };

    container(
        column![
            container(row![text("NOTES").size(10).color(theme::TEXT_MUTED),].padding([0, 0]),),
            inner,
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([14, 18]),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(2))
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(theme::BG_DARK)),
        border: Border {
            color: theme::BORDER_PANEL,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}
