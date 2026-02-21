use crate::slides::{Background, Presentation};
use crate::ui::components::{group_label_widget, truncate};
use crate::ui::messages::Message;
use crate::ui::theme;
use iced_font_awesome::fa_icon_solid;
use iced::{
    Alignment, Color, Element, Length,
    widget::{Column, Id, Space, button, column, container, row, scrollable, text},
};
use std::sync::OnceLock;

static SLIDE_LIST_SCROLL: OnceLock<Id> = OnceLock::new();

pub fn scrollable_id() -> Id {
    SLIDE_LIST_SCROLL.get_or_init(Id::unique).clone()
}

pub const PANEL_WIDTH: f32 = 280.0;
const THUMB_W: f32 = 118.0;
const THUMB_H: f32 = 66.0;
const ROW_PAD: u16 = 6;
const COL_GAP: u16 = 6;


pub fn slide_list<'a>(
    presentation: &'a Presentation,
    selected_index: Option<usize>,
) -> Element<'a, Message> {
    let add_msg = match selected_index {
        Some(i) => Message::AddSlideAfter(i),
        None => Message::AddSlide,
    };

    let header = container(
        row![
            text("SLIDES").size(10).color(theme::TEXT_MUTED),
            Space::new().width(Length::Fill),
            button(text("+ Add").size(11))
                .on_press(add_msg)
                .padding([4, 12])
                .style(theme::primary_button),
        ]
        .padding([8, 12])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::section_header_style);

    let mut outer_col = Column::new().spacing(0).padding([4, 4]);

    if presentation.slides.is_empty() {
        outer_col = outer_col.push(
            container(text("No slides yet").size(12).color(theme::TEXT_MUTED))
                .padding(20)
                .center_x(Length::Fill),
        );
    } else {
        let total = presentation.slides.len();

        let mut groups: Vec<(Option<String>, usize, usize)> = Vec::new();
        for (i, slide) in presentation.slides.iter().enumerate() {
            let lbl = slide.group_label.clone();
            if let Some(last) = groups.last_mut()
                && last.0 == lbl
            {
                last.2 += 1;
                continue;
            }
            groups.push((lbl, i, 1));
        }

        for (group_label, start, count) in groups {
            if let Some(ref lbl) = group_label {
                outer_col = outer_col.push(Space::new().height(6));
                outer_col = outer_col.push(group_label_widget(lbl));
                outer_col = outer_col.push(Space::new().height(2));
            }

            let end = start + count;
            let mut idx = start;
            while idx < end {
                let left_cell = slide_cell(presentation, idx, selected_index, total);
                let right_cell = if idx + 1 < end {
                    Some(slide_cell(presentation, idx + 1, selected_index, total))
                } else {
                    None
                };

                let grid_row = {
                    let left_wrap = container(left_cell).width(THUMB_W).height(Length::Shrink);
                    let right_wrap: Element<'a, Message> = if let Some(rc) = right_cell {
                        container(rc).width(THUMB_W).height(Length::Shrink).into()
                    } else {
                        Space::new().width(THUMB_W).into()
                    };
                    row![left_wrap, right_wrap]
                        .spacing(f32::from(COL_GAP))
                        .padding([0, ROW_PAD])
                };

                outer_col = outer_col.push(grid_row);
                outer_col = outer_col.push(Space::new().height(4));

                idx += 2;
            }
        }
    }

    let panel = column![
        header,
        scrollable(outer_col)
            .id(scrollable_id())
            .height(Length::Fill)
    ];
    container(panel)
        .width(PANEL_WIDTH)
        .height(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn slide_cell<'a>(
    presentation: &'a Presentation,
    index: usize,
    selected_index: Option<usize>,
    total: usize,
) -> Element<'a, Message> {
    let slide = &presentation.slides[index];
    let is_selected = selected_index == Some(index);

    let bg_color = match &slide.background {
        Background::Solid(c) => Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0),
        _ => Color::BLACK,
    };

    let preview: String = match &slide.content {
        crate::slides::SlideContent::Text { text, .. } => {
            if text.is_empty() {
                "(Empty)".into()
            } else {
                truncate(text, 30)
            }
        }
        crate::slides::SlideContent::Image { .. } => "📷 Image".into(),
        crate::slides::SlideContent::Video { .. } => "🎬 Video".into(),
    };

    let txt_color = if let crate::slides::SlideContent::Text { style, .. } = &slide.content {
        Color::from_rgba8(
            style.color.r,
            style.color.g,
            style.color.b,
            style.color.a as f32 / 255.0,
        )
    } else {
        Color::WHITE
    };

    let thumb = container(text(preview).size(7).color(txt_color))
        .width(THUMB_W)
        .height(THUMB_H)
        .padding(5)
        .center(Length::Fill)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(bg_color)),
            border: iced::Border {
                color: if is_selected {
                    theme::ACCENT_BLUE
                } else {
                    theme::BORDER_STRONG
                },
                width: if is_selected { 2.0 } else { 1.0 },
                radius: 4.0.into(),
            },
            ..Default::default()
        });

    let thumb_btn = button(thumb)
        .on_press(Message::SelectSlide(index))
        .padding(0)
        .style(theme::invisible_button);

    let num_label = text(format!("{}", index + 1))
        .size(9)
        .color(if is_selected {
            theme::ACCENT_BLUE
        } else {
            theme::TEXT_MUTED
        });

    let can_up = index > 0;
    let can_down = index < total - 1;

    let up_btn = {
        let b = button(fa_icon_solid("arrow-up").size(10.0).color(theme::TEXT_MUTED))
            .padding([1, 4])
            .style(theme::ghost_button);
        if can_up {
            b.on_press(Message::MoveSlideUp(index))
        } else {
            b
        }
    };
    let down_btn = {
        let b = button(fa_icon_solid("arrow-down").size(10.0).color(theme::TEXT_MUTED))
            .padding([1, 4])
            .style(theme::ghost_button);
        if can_down {
            b.on_press(Message::MoveSlideDown(index))
        } else {
            b
        }
    };
    let dup_btn = button(text("⧉").size(8).color(theme::TEXT_MUTED))
        .on_press(Message::DuplicateSlide(index))
        .padding([1, 4])
        .style(theme::ghost_button);
    let del_btn = button(text("X").size(8).color(theme::DANGER_RED))
        .on_press(Message::DeleteSlide(slide.id.clone()))
        .padding([1, 4])
        .style(theme::ghost_button);

    let actions = row![
        up_btn,
        down_btn,
        dup_btn,
        Space::new().width(Length::Fill),
        del_btn
    ]
    .spacing(1)
    .width(THUMB_W);

    column![num_label, thumb_btn, actions]
        .spacing(2)
        .width(THUMB_W)
        .into()
}
