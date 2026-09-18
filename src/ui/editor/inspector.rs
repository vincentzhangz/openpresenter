use crate::domain::{
    Background as SlideBackground, Color as SlideColor, ImageFit, ObjectContent, ShapeType, Slide,
    SlideContent, SlideTheme, TextAlignment, TextStyle, TextTransform, Transition,
};
use crate::ui::components::{
    color_channel_slider, color_swatch_btn, compact_toggle_btn, option_btn, section_label, tab_bar,
    tab_btn,
};
use crate::ui::messages::{InspectorTab, Message};
use crate::ui::theme;
use crate::ui::{layers, slides, typography};
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{
        Column, Row, Space, button, checkbox, column, container, row, scrollable, slider, text,
        text_input,
    },
};
use iced_font_awesome::fa_icon_solid;

pub const PANEL_WIDTH: u16 = 320;

#[derive(Clone, Default)]
pub struct LayerPanelState {
    pub selected_layer_index: Option<usize>,
    pub editing_text: String,
    pub editing_font_size: String,
    pub editing_pos_x: String,
    pub editing_pos_y: String,
    pub editing_width: String,
    pub editing_height: String,
    pub editing_stroke_width: String,
    pub editing_font_family: String,
    pub editing_line_height: String,
    pub editing_letter_spacing: String,
    pub editing_glow_radius: String,
    pub editing_text_stroke_width: String,
    pub can_paste_style: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn inspector_panel<'a>(
    slide: Option<&'a Slide>,
    tab: InspectorTab,
    editing_text: &'a str,
    editing_font_size: &'a str,
    editing_transition_dur: &'a str,
    editing_group_label: &'a str,
    editing_notes: &'a str,
    themes: &'a [SlideTheme],
    selected_theme_id: Option<&'a str>,
    new_theme_name: &'a str,
    video_playing: bool,
    video_looping: bool,
    video_volume: f32,
    video_muted: bool,
    video_speed: f64,
    video_position: f64,
    video_duration: f64,
    layer_state: &LayerPanelState,
) -> Element<'a, Message> {
    let tab_bar_el = tab_bar(vec![
        tab_btn(
            "Shape",
            tab == InspectorTab::Layers,
            Message::SwitchInspectorTab(InspectorTab::Layers),
        ),
        tab_btn(
            "Text",
            tab == InspectorTab::Text,
            Message::SwitchInspectorTab(InspectorTab::Text),
        ),
        tab_btn(
            "Build",
            tab == InspectorTab::Slide,
            Message::SwitchInspectorTab(InspectorTab::Slide),
        ),
        tab_btn(
            "Theme",
            tab == InspectorTab::Theme,
            Message::SwitchInspectorTab(InspectorTab::Theme),
        ),
    ]);

    let panel_content: Element<'a, Message> = match (tab, slide) {
        (InspectorTab::Theme, _) => {
            themes_tab(themes, selected_theme_id, new_theme_name, slide).into()
        }
        (InspectorTab::Text, Some(slide)) => text_tab(
            slide,
            editing_text,
            editing_font_size,
            layer_state.selected_layer_index,
            layer_state.can_paste_style,
            video_playing,
            video_looping,
            video_volume,
            video_muted,
            video_speed,
            video_position,
            video_duration,
        )
        .into(),
        (InspectorTab::Slide, Some(slide)) => slide_tab(
            slide,
            editing_transition_dur,
            editing_group_label,
            editing_notes,
        )
        .into(),
        (InspectorTab::Layers, Some(slide)) => layers_tab(slide, layer_state).into(),
        _ => container(
            text("Select a slide to edit properties")
                .size(13)
                .color(theme::TEXT_MUTED),
        )
        .padding([20, 14])
        .into(),
    };

    let scroll = scrollable(panel_content).height(Length::Fill);

    let mut inner = column![tab_bar_el, scroll];

    if slide.is_some() {
        inner = inner.push(
            container(
                button(text("Save Slide").size(13))
                    .on_press(Message::from(slides::Message::SaveSlide))
                    .width(Length::Fill)
                    .padding([9, 0])
                    .style(theme::primary_button),
            )
            .padding([10, 14]),
        );
    }

    container(inner)
        .width(PANEL_WIDTH as f32)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

#[allow(clippy::too_many_arguments)]
fn text_tab<'a>(
    slide: &'a Slide,
    editing_text: &'a str,
    editing_font_size: &'a str,
    selected_layer_index: Option<usize>,
    can_paste_style: bool,
    video_playing: bool,
    video_looping: bool,
    video_volume: f32,
    video_muted: bool,
    video_speed: f64,
    video_position: f64,
    video_duration: f64,
) -> Column<'a, Message> {
    let selected_layer_text = selected_layer_index.and_then(|idx| {
        slide.layers.get(idx).and_then(|l| match &l.content {
            ObjectContent::Text { text, style, .. } => Some((text.as_str(), style)),
            _ => None,
        })
    });

    let slide_content_text = match &slide.content {
        SlideContent::Text { text, style } => Some((text.as_str(), style)),
        _ => None,
    };

    let first_text_layer = slide.layers.iter().find_map(|l| match &l.content {
        ObjectContent::Text { text, style, .. } => Some((text.as_str(), style)),
        _ => None,
    });

    if let Some((_txt, style)) = selected_layer_text
        .or(slide_content_text)
        .or(first_text_layer)
    {
        let current_text = if !editing_text.is_empty() {
            editing_text
        } else {
            _txt
        };

        let current_fs = if !editing_font_size.is_empty() {
            editing_font_size
        } else {
            "70"
        };

        text_content_tab(style, current_text, current_fs, can_paste_style)
    } else {
        match &slide.content {
            SlideContent::Image { path, fit } => image_content_tab(path, *fit),
            SlideContent::Video { path, thumbnail } => video_content_tab(
                path,
                thumbnail.as_deref(),
                video_playing,
                video_looping,
                video_volume,
                video_muted,
                video_speed,
                video_position,
                video_duration,
            ),
            _ => {
                let mut panel = Column::new().padding([10, 10]).spacing(10);
                panel = panel.push(section_label("TEXT CONTENT"));
                panel = panel.push(
                    text_input("Enter slide text…", editing_text)
                        .on_input(|v| Message::from(slides::Message::SlideTextChanged(v)))
                        .padding([9, 10])
                        .size(13),
                );
                panel
            }
        }
    }
}

fn text_content_tab<'a>(
    style: &'a TextStyle,
    editing_text: &'a str,
    editing_font_size: &'a str,
    can_paste_style: bool,
) -> Column<'a, Message> {
    let mut panel = Column::new().padding([10, 10]).spacing(10);

    panel = panel.push(section_label("TEXT CONTENT"));
    panel = panel.push(
        text_input("Enter slide text…", editing_text)
            .on_input(|v| Message::from(slides::Message::SlideTextChanged(v)))
            .padding([9, 10])
            .size(13),
    );

    let divider = || {
        container(Space::new().width(Length::Fill).height(1))
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::BORDER_PANEL)),
                ..Default::default()
            })
            .padding([3, 0])
    };

    let copy_btn = button(text("Copy Style").size(12))
        .on_press(Message::from(slides::Message::CopyTextStyle))
        .width(Length::FillPortion(1))
        .padding([7, 10])
        .style(theme::secondary_button);

    let mut paste_btn = button(text("Paste Style").size(12).color(if can_paste_style {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    }))
    .width(Length::FillPortion(1))
    .padding([7, 10])
    .style(if can_paste_style {
        theme::secondary_button
    } else {
        theme::ghost_button
    });

    if can_paste_style {
        paste_btn = paste_btn.on_press(Message::from(slides::Message::PasteTextStyle));
    }

    panel = panel.push(row![copy_btn, paste_btn].spacing(8));

    panel = panel.push(divider());

    panel = panel.push(faux_dropdown_btn(
        style.font_family.as_str(),
        Message::from(slides::Message::SlideFontFamilyCycle),
    ));

    panel = panel.push(
        row![
            faux_dropdown_btn(
                if style.bold { "Bold" } else { "Regular" },
                Message::from(slides::Message::SlideBoldToggled(!style.bold)),
            ),
            text_input("70", editing_font_size)
                .on_input(|v| Message::from(slides::Message::SlideFontSizeChanged(v)))
                .width(70)
                .padding([7, 8])
                .size(12),
            button(fa_icon_solid("minus").size(10.0_f32))
                .on_press(Message::from(slides::Message::SlideFontSizeStep(-4.0)))
                .padding([6, 7])
                .style(theme::secondary_button),
            button(fa_icon_solid("plus").size(10.0_f32))
                .on_press(Message::from(slides::Message::SlideFontSizeStep(4.0)))
                .padding([6, 7])
                .style(theme::secondary_button),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    );

    panel = panel.push(
        row![
            icon_toggle_btn(
                "bold",
                style.bold,
                Message::from(slides::Message::SlideBoldToggled(!style.bold))
            ),
            icon_toggle_btn(
                "italic",
                style.italic,
                Message::from(slides::Message::SlideItalicToggled(!style.italic))
            ),
            icon_toggle_btn(
                "underline",
                style.outline,
                Message::from(slides::Message::SlideOutlineToggled(!style.outline))
            ),
            icon_toggle_btn(
                "strikethrough",
                style.shadow,
                Message::from(slides::Message::SlideShadowToggled(!style.shadow))
            ),
            Space::new().width(Length::Fill),
            button(
                fa_icon_solid("sun")
                    .size(11.0_f32)
                    .color(if style.glow_enabled {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TEXT_SECONDARY
                    })
            )
            .on_press(Message::from(slides::Message::SlideGlowToggled(
                !style.glow_enabled
            )))
            .padding([6, 10])
            .style(theme::secondary_button),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    );

    let cap_label = match style.text_transform {
        crate::domain::TextTransform::None => "None",
        crate::domain::TextTransform::Uppercase => "All Caps",
        crate::domain::TextTransform::Lowercase => "Lower Case",
        crate::domain::TextTransform::Capitalize => "Title Case",
    };

    panel = panel.push(
        row![
            text("Capitalization").size(12).color(theme::TEXT_SECONDARY),
            Space::new().width(Length::Fill),
            faux_dropdown_btn(
                cap_label,
                Message::from(slides::Message::SlideTextTransformCycle),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    panel = panel.push(divider());

    let current_color = Color::from_rgba8(
        style.color.r,
        style.color.g,
        style.color.b,
        style.color.a as f32 / 255.0,
    );
    panel = panel.push(
        row![
            text("Text Color").size(13).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            button(Space::new().width(60).height(22))
                .on_press(Message::from(slides::Message::SlideTextColorCycle))
                .style(theme::swatch_button(current_color, false)),
            button(fa_icon_solid("palette").size(13.0_f32))
                .on_press(Message::from(slides::Message::SlideTextColorCycle))
                .padding([5, 8])
                .style(theme::secondary_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    );

    let mut color_row = Row::new().spacing(4).align_y(Alignment::Center);
    for &preset in crate::ui::slides::COLOR_PRESETS {
        let c = Color::from_rgba8(preset.r, preset.g, preset.b, preset.a as f32 / 255.0);
        let is_active = preset == style.color;
        color_row = color_row.push(
            button(Space::new().width(24).height(18))
                .on_press(Message::from(slides::Message::SlideColorChanged(preset)))
                .style(theme::swatch_button(c, is_active)),
        );
    }
    panel = panel.push(color_row);

    panel = panel.push(divider());

    panel = panel.push(
        row![
            align_icon_btn("align-left", TextAlignment::Left, &style.alignment),
            align_icon_btn("align-center", TextAlignment::Center, &style.alignment),
            align_icon_btn("align-right", TextAlignment::Right, &style.alignment),
        ]
        .spacing(4),
    );

    let top_active = style.position_y <= 0.30;
    let middle_active = (style.position_y - 0.5).abs() < 0.20;
    let bottom_active = style.position_y >= 0.70;
    panel = panel.push(
        row![
            icon_toggle_btn(
                "arrow-up",
                top_active,
                Message::from(slides::Message::SlidePositionPreset(style.position_x, 0.15))
            ),
            icon_toggle_btn(
                "up-down",
                middle_active,
                Message::from(slides::Message::SlidePositionPreset(style.position_x, 0.5))
            ),
            icon_toggle_btn(
                "arrow-down",
                bottom_active,
                Message::from(slides::Message::SlidePositionPreset(style.position_x, 0.85))
            ),
        ]
        .spacing(4),
    );

    panel = panel.push(divider());
    panel = panel.push(
        row![
            checkbox(style.outline)
                .on_toggle(|v| Message::from(slides::Message::SlideOutlineToggled(v))),
            text("Stroke").size(12).color(theme::TEXT_PRIMARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    panel = panel.push(divider());
    panel = panel.push(
        row![
            checkbox(style.shadow)
                .on_toggle(|v| Message::from(slides::Message::SlideShadowToggled(v))),
            text("Shadow").size(12).color(theme::TEXT_PRIMARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    panel = panel.push(divider());
    panel = panel.push(
        row![
            checkbox(style.glow_enabled)
                .on_toggle(|v| Message::from(slides::Message::SlideGlowToggled(v))),
            text("Glow Effect").size(12).color(theme::TEXT_PRIMARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    panel = panel.push(Space::new().height(40));

    panel
}

fn slide_tab<'a>(
    slide: &'a Slide,
    editing_transition_dur: &'a str,
    editing_group_label: &'a str,
    editing_notes: &'a str,
) -> Column<'a, Message> {
    let mut panel = Column::new().padding([14, 14]).spacing(12);

    panel = panel.push(section_label("CONTENT TYPE"));
    let is_text = matches!(slide.content, SlideContent::Text { .. });
    let is_image = matches!(slide.content, SlideContent::Image { .. });
    let is_video = matches!(slide.content, SlideContent::Video { .. });
    panel = panel.push(
        row![
            content_type_btn(
                "Text",
                Message::from(slides::Message::ConvertSlideToText),
                is_text
            ),
            content_type_btn(
                "Image",
                Message::from(slides::Message::ConvertSlideToImage),
                is_image
            ),
            content_type_btn(
                "Video",
                Message::from(slides::Message::ConvertSlideToVideo),
                is_video
            ),
        ]
        .spacing(4),
    );

    panel = panel.push(section_label("BACKGROUND"));
    panel = panel.push(color_swatch_row_bg(&slide.background));

    panel = panel.push(section_label("TRANSITION"));

    let is_cut = matches!(slide.transition, Transition::Cut);
    let is_fade = matches!(slide.transition, Transition::Fade { .. });
    let is_slide_t = matches!(slide.transition, Transition::Slide { .. });
    let is_dissolve = matches!(slide.transition, Transition::Dissolve { .. });
    let is_push = matches!(slide.transition, Transition::Push { .. });
    let is_zoom = matches!(slide.transition, Transition::Zoom { .. });
    let is_flip = matches!(slide.transition, Transition::Flip { .. });
    let is_clock = matches!(slide.transition, Transition::Clock { .. });
    let is_wipe = matches!(slide.transition, Transition::Wipe { .. });

    let dur_ms: u64 = match slide.transition {
        Transition::Cut => 500,
        Transition::Fade { duration_ms }
        | Transition::Dissolve { duration_ms }
        | Transition::Slide { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Wipe { duration_ms, .. } => duration_ms,
    };

    panel = panel.push(
        row![
            transition_btn("Cut", Transition::Cut, is_cut),
            transition_btn(
                "Fade",
                Transition::Fade {
                    duration_ms: dur_ms
                },
                is_fade
            ),
            transition_btn(
                "Dissolve",
                Transition::Dissolve {
                    duration_ms: dur_ms
                },
                is_dissolve
            ),
            transition_btn(
                "Slide",
                Transition::Slide {
                    duration_ms: dur_ms
                },
                is_slide_t
            ),
        ]
        .spacing(4),
    );
    panel = panel.push(
        row![
            transition_btn(
                "Push",
                Transition::Push {
                    duration_ms: dur_ms,
                    direction: 1
                },
                is_push
            ),
            transition_btn(
                "Zoom",
                Transition::Zoom {
                    duration_ms: dur_ms
                },
                is_zoom
            ),
            transition_btn(
                "Flip",
                Transition::Flip {
                    duration_ms: dur_ms
                },
                is_flip
            ),
            transition_btn(
                "Clock",
                Transition::Clock {
                    duration_ms: dur_ms
                },
                is_clock
            ),
            transition_btn(
                "Wipe",
                Transition::Wipe {
                    duration_ms: dur_ms,
                    angle_deg: 0
                },
                is_wipe
            ),
        ]
        .spacing(4),
    );

    if !is_cut {
        panel = panel.push(
            row![
                text("Duration (ms)")
                    .size(11)
                    .color(theme::TEXT_SECONDARY)
                    .width(90),
                text_input("500", editing_transition_dur)
                    .on_input(|v| Message::from(slides::Message::TransitionDurationChanged(v)))
                    .width(70)
                    .padding([7, 8])
                    .size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    panel = panel.push(section_label("GROUP LABEL"));
    panel = panel.push(
        text_input("e.g. Verse 1", editing_group_label)
            .on_input(|v| Message::from(slides::Message::GroupLabelChanged(v)))
            .padding([7, 8])
            .size(12)
            .width(Length::Fill),
    );

    panel = panel.push(section_label("NOTES"));
    panel = panel.push(
        text_input("Operator notes (shown on stage display)", editing_notes)
            .on_input(|v| Message::from(slides::Message::SlideNotesChanged(v)))
            .padding([7, 8])
            .size(12)
            .width(Length::Fill),
    );

    panel
}

fn speed_btn<'a>(label: &'a str, speed: f64, current_speed: f64) -> Element<'a, Message> {
    let active = (current_speed - speed).abs() < 0.05;
    option_btn(
        label,
        active,
        Message::Video(crate::ui::video::Message::SpeedChanged(speed)),
    )
}

fn content_type_btn<'a>(label: &'a str, msg: Message, active: bool) -> Element<'a, Message> {
    option_btn(label, active, msg)
}

fn image_content_tab<'a>(path: &'a str, fit: ImageFit) -> Column<'a, Message> {
    let mut panel = Column::new().padding([14, 14]).spacing(12);

    panel = panel.push(section_label("IMAGE FILE"));
    let display = if path.is_empty() {
        "(no file selected)".to_string()
    } else {
        std::path::Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string())
    };
    panel = panel.push(
        row![
            text(display)
                .size(11)
                .color(theme::TEXT_SECONDARY)
                .width(Length::Fill),
            button(text("Browse…").size(11))
                .on_press(Message::from(slides::Message::PickImageFile))
                .padding([6, 12])
                .style(theme::secondary_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    );

    panel = panel.push(section_label("IMAGE FIT"));
    panel = panel.push(
        row![
            fit_btn("Fit", ImageFit::Fit, fit),
            fit_btn("Fill", ImageFit::Fill, fit),
            fit_btn("Stretch", ImageFit::Stretch, fit),
            fit_btn("Center", ImageFit::Center, fit),
        ]
        .spacing(4),
    );

    panel
}

fn fit_btn<'a>(label: &'a str, this: ImageFit, current: ImageFit) -> Element<'a, Message> {
    option_btn(
        label,
        this == current,
        Message::from(slides::Message::SlideImageFitChanged(this)),
    )
}

#[allow(clippy::too_many_arguments)]
fn video_content_tab<'a>(
    path: &'a str,
    thumbnail: Option<&'a str>,
    video_playing: bool,
    video_looping: bool,
    video_volume: f32,
    video_muted: bool,
    video_speed: f64,
    video_position: f64,
    video_duration: f64,
) -> Column<'a, Message> {
    let mut panel = Column::new().padding([14, 14]).spacing(12);

    panel = panel.push(section_label("VIDEO FILE"));
    let display = if path.is_empty() {
        "(no file selected)".to_string()
    } else {
        std::path::Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string())
    };
    panel = panel.push(
        row![
            text(display)
                .size(11)
                .color(theme::TEXT_SECONDARY)
                .width(Length::Fill),
            button(text("Browse…").size(11))
                .on_press(Message::from(slides::Message::PickVideoFile))
                .padding([6, 12])
                .style(theme::secondary_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    );

    if !path.is_empty() {
        panel = panel.push(section_label("PLAYBACK"));

        panel = panel.push(
            row![
                button(fa_icon_solid(if video_playing { "pause" } else { "play" }).size(12.0_f32))
                    .on_press(Message::Video(crate::ui::video::Message::PlayToggled))
                    .padding([6, 14])
                    .style(theme::primary_button),
                Space::new().width(8),
                button(fa_icon_solid("repeat").size(12.0_f32))
                    .on_press(Message::Video(crate::ui::video::Message::LoopToggled(
                        !video_looping
                    )))
                    .padding([6, 12])
                    .style(if video_looping {
                        theme::primary_button
                    } else {
                        theme::secondary_button
                    }),
            ]
            .align_y(Alignment::Center),
        );

        panel = panel.push(section_label("VOLUME"));
        panel = panel.push(
            row![
                button(
                    fa_icon_solid(if video_muted {
                        "volume-xmark"
                    } else {
                        "volume-high"
                    })
                    .size(12.0_f32)
                )
                .on_press(Message::Video(crate::ui::video::Message::MuteToggled))
                .padding([5, 8])
                .style(if video_muted {
                    theme::primary_button
                } else {
                    theme::secondary_button
                }),
                Space::new().width(6),
                slider(0.0..=1.0, video_volume, move |v| Message::Video(
                    crate::ui::video::Message::VolumeChanged(v)
                ))
                .step(0.05_f32)
                .width(Length::Fill),
                Space::new().width(6),
                text(format!("{:.0}%", video_volume * 100.0))
                    .size(11)
                    .color(theme::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        );

        if video_duration > 0.0 {
            panel = panel.push(section_label("POSITION"));
            let safe_pos = video_position.clamp(0.0, video_duration);
            panel = panel.push(
                row![
                    slider(0.0..=video_duration, safe_pos, move |v| Message::Video(
                        crate::ui::video::Message::SeekChanged(v)
                    ))
                    .step(0.5)
                    .width(Length::Fill),
                    Space::new().width(6),
                    text(format!(
                        "{:.0}:{:02.0} / {:.0}:{:02.0}",
                        (safe_pos / 60.0).floor(),
                        safe_pos % 60.0,
                        (video_duration / 60.0).floor(),
                        video_duration % 60.0,
                    ))
                    .size(10)
                    .color(theme::TEXT_SECONDARY),
                ]
                .align_y(Alignment::Center),
            );
        }

        panel = panel.push(section_label("SPEED"));
        panel = panel.push(
            row![
                speed_btn("0.5x", 0.5, video_speed),
                Space::new().width(4),
                speed_btn("1x", 1.0, video_speed),
                Space::new().width(4),
                speed_btn("1.5x", 1.5, video_speed),
                Space::new().width(4),
                speed_btn("2x", 2.0, video_speed),
            ]
            .align_y(Alignment::Center),
        );
    }

    if let Some(thumb) = thumbnail.filter(|t| !t.is_empty()) {
        panel = panel.push(section_label("THUMBNAIL"));
        let thumb_name = std::path::Path::new(thumb)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| thumb.to_string());
        panel = panel.push(text(thumb_name).size(10).color(theme::TEXT_MUTED));
    }

    panel
}

fn themes_tab<'a>(
    themes: &'a [SlideTheme],
    selected_theme_id: Option<&'a str>,
    new_theme_name: &'a str,
    slide: Option<&'a Slide>,
) -> Column<'a, Message> {
    let has_text = slide
        .map(|s| matches!(s.content, SlideContent::Text { .. }))
        .unwrap_or(false);
    let mut panel = Column::new().padding([14, 14]).spacing(12);

    panel = panel.push(section_label("SAVE CURRENT SLIDE AS THEME"));
    panel = panel.push(
        row![
            text_input("Theme name…", new_theme_name)
                .on_input(|v| Message::Themes(crate::ui::themes::Message::NameChanged(v)))
                .padding([7, 8])
                .size(12)
                .width(Length::Fill),
            button(text("Save").size(11))
                .on_press_maybe(if !new_theme_name.trim().is_empty() && has_text {
                    Some(Message::Themes(
                        crate::ui::themes::Message::SaveSlideAsTheme,
                    ))
                } else {
                    None
                })
                .padding([7, 10])
                .style(theme::secondary_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    );
    if !has_text {
        panel = panel.push(
            text("Select a text slide to save as theme.")
                .size(10)
                .color(theme::TEXT_MUTED),
        );
    }

    panel = panel.push(section_label("IMPORT / EXPORT"));
    panel = panel.push(
        row![
            button(text("Export JSON").size(11))
                .on_press(Message::Themes(crate::ui::themes::Message::Export))
                .padding([6, 10])
                .style(theme::secondary_button),
            button(text("Import JSON").size(11))
                .on_press(Message::Themes(crate::ui::themes::Message::Import))
                .padding([6, 10])
                .style(theme::secondary_button),
        ]
        .spacing(6),
    );

    panel = panel.push(section_label("SAVED THEMES"));

    if themes.is_empty() {
        panel = panel.push(
            text("No themes saved yet.")
                .size(11)
                .color(theme::TEXT_MUTED),
        );
    } else {
        for theme_item in themes {
            let is_selected = selected_theme_id == Some(theme_item.id.as_str());

            let bg_iced = match &theme_item.background {
                SlideBackground::Solid(c) => Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0),
                SlideBackground::Image(_) => Color::from_rgb(0.3, 0.3, 0.5),
                SlideBackground::Video(_) => Color::from_rgb(0.2, 0.2, 0.4),
            };

            let swatch = button(Space::new().width(22).height(22))
                .padding(0)
                .style(theme::swatch_button(bg_iced, is_selected));

            let name_label = text(theme_item.name.as_str())
                .size(12)
                .color(if is_selected {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_SECONDARY
                })
                .width(Length::Fill);

            let apply_btn = button(text("Apply").size(10))
                .on_press(Message::Themes(crate::ui::themes::Message::Apply(
                    theme_item.id.clone(),
                )))
                .padding([4, 8])
                .style(theme::primary_button);

            let delete_btn = button(fa_icon_solid("xmark").size(13.0_f32))
                .on_press(Message::Themes(crate::ui::themes::Message::Delete(
                    theme_item.id.clone(),
                )))
                .padding([3, 7])
                .style(theme::danger_button);

            let select_msg =
                Message::Themes(crate::ui::themes::Message::Select(theme_item.id.clone()));
            let row = button(
                row![swatch, name_label, apply_btn, delete_btn]
                    .spacing(6)
                    .align_y(Alignment::Center),
            )
            .on_press(select_msg)
            .padding([6, 4])
            .style(if is_selected {
                theme::primary_button
            } else {
                theme::ghost_button
            });

            panel = panel.push(row);
        }
    }

    panel
}

fn faux_dropdown_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(
        row![
            text(label).size(12).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            fa_icon_solid("chevron-down")
                .size(10.0_f32)
                .color(theme::TEXT_SECONDARY),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(msg)
    .width(Length::Fill)
    .padding([6, 10])
    .style(theme::secondary_button)
    .into()
}

fn icon_toggle_btn<'a>(icon: &'a str, active: bool, msg: Message) -> Element<'a, Message> {
    button(fa_icon_solid(icon).size(11.0_f32).color(if active {
        Color::WHITE
    } else {
        theme::TEXT_SECONDARY
    }))
    .on_press(msg)
    .padding([6, 16])
    .style(if active {
        theme::primary_button
    } else {
        theme::secondary_button
    })
    .into()
}

fn align_icon_btn<'a>(
    icon: &'a str,
    this: TextAlignment,
    current: &TextAlignment,
) -> Element<'a, Message> {
    let active = std::mem::discriminant(&this) == std::mem::discriminant(current);
    let color = if active {
        Color::WHITE
    } else {
        theme::TEXT_SECONDARY
    };
    button(fa_icon_solid(icon).size(11.0_f32).color(color))
        .on_press(Message::from(slides::Message::SlideAlignmentChanged(this)))
        .padding([5, 10])
        .style(if active {
            theme::primary_button
        } else {
            theme::secondary_button
        })
        .into()
}

fn transition_btn<'a>(label: &'a str, t: Transition, active: bool) -> Element<'a, Message> {
    option_btn(
        label,
        active,
        Message::from(slides::Message::SlideTransitionChanged(t)),
    )
}

fn color_swatch_row_bg<'a>(current: &'a SlideBackground) -> Element<'a, Message> {
    let current_color = match current {
        SlideBackground::Solid(c) => Some(*c),
        _ => None,
    };

    let swatches: Vec<(Color, SlideColor)> = vec![
        (Color::BLACK, SlideColor::black()),
        (Color::WHITE, SlideColor::white()),
        (
            Color::from_rgb(0.1, 0.1, 0.4),
            SlideColor {
                r: 25,
                g: 25,
                b: 100,
                a: 255,
            },
        ),
        (
            Color::from_rgb(0.1, 0.3, 0.1),
            SlideColor {
                r: 25,
                g: 75,
                b: 25,
                a: 255,
            },
        ),
        (
            Color::from_rgb(0.4, 0.1, 0.1),
            SlideColor {
                r: 100,
                g: 25,
                b: 25,
                a: 255,
            },
        ),
        (
            Color::from_rgb(0.2, 0.2, 0.35),
            SlideColor {
                r: 50,
                g: 50,
                b: 90,
                a: 255,
            },
        ),
    ];

    let mut r = row![].spacing(6);
    for (iced_color, slide_color) in swatches {
        let selected = current_color
            .map(|c| c.r == slide_color.r && c.g == slide_color.g && c.b == slide_color.b)
            .unwrap_or(false);
        r = r.push(color_swatch_btn(
            iced_color,
            selected,
            Message::from(slides::Message::SlideBackgroundChanged(
                SlideBackground::Solid(slide_color),
            )),
        ));
    }
    r.into()
}

fn layers_tab<'a>(slide: &'a Slide, state: &LayerPanelState) -> Column<'a, Message> {
    let mut col = Column::new().spacing(0);

    let add_row = container(
        row![
            button(text("+ Text").size(11))
                .on_press(Message::from(layers::Message::AddTextLayer))
                .style(theme::ghost_button)
                .padding([4, 8]),
            button(text("+ Rect").size(11))
                .on_press(Message::from(layers::Message::AddShapeLayer(
                    ShapeType::Rectangle
                )))
                .style(theme::ghost_button)
                .padding([4, 8]),
            button(text("+ Ellipse").size(11))
                .on_press(Message::from(layers::Message::AddShapeLayer(
                    ShapeType::Ellipse
                )))
                .style(theme::ghost_button)
                .padding([4, 8]),
        ]
        .spacing(4)
        .padding([6, 10]),
    )
    .width(Length::Fill)
    .style(theme::section_header_style);
    col = col.push(add_row);

    let layers = &slide.layers;

    if layers.is_empty() {
        col = col.push(
            container(
                text("No layers — add one above, or\nclick \"+ Text\" to get started.")
                    .size(12)
                    .color(theme::TEXT_MUTED),
            )
            .padding([16, 14]),
        );
    } else {
        let mut sorted: Vec<(usize, &crate::domain::Object)> = layers.iter().enumerate().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1.z_order));

        for (orig_idx, layer) in sorted {
            let is_sel = state.selected_layer_index == Some(orig_idx);
            let vis_btn = button(
                fa_icon_solid(if layer.visible { "eye" } else { "eye-slash" })
                    .size(11.0_f32)
                    .color(if layer.visible {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_MUTED
                    }),
            )
            .on_press(Message::from(layers::Message::ToggleLayerVisibility(
                orig_idx,
            )))
            .style(theme::ghost_button)
            .padding([2, 4]);

            let lock_btn = button(
                fa_icon_solid(if layer.locked { "lock" } else { "lock-open" })
                    .size(11.0_f32)
                    .color(if layer.locked {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TEXT_MUTED
                    }),
            )
            .on_press(Message::from(layers::Message::ToggleLayerLock(orig_idx)))
            .style(theme::ghost_button)
            .padding([2, 4]);

            let name_color = if is_sel {
                theme::TEXT_PRIMARY
            } else if layer.visible {
                theme::TEXT_SECONDARY
            } else {
                theme::TEXT_MUTED
            };

            let name_btn = button(text(layer.display_name()).size(12).color(name_color))
                .on_press(Message::from(layers::Message::SelectLayer(Some(orig_idx))))
                .width(Length::Fill)
                .style(theme::ghost_button)
                .padding([2, 4]);

            let up_btn = button(fa_icon_solid("arrow-up").size(10.0_f32))
                .on_press(Message::from(layers::Message::MoveLayerUp(orig_idx)))
                .style(theme::ghost_button)
                .padding([2, 4]);

            let down_btn = button(fa_icon_solid("arrow-down").size(10.0_f32))
                .on_press(Message::from(layers::Message::MoveLayerDown(orig_idx)))
                .style(theme::ghost_button)
                .padding([2, 4]);

            let del_btn = button(
                fa_icon_solid("xmark")
                    .size(13.0_f32)
                    .color(theme::DANGER_RED),
            )
            .on_press(Message::from(layers::Message::DeleteLayer(orig_idx)))
            .style(theme::ghost_button)
            .padding([2, 6]);

            let row_el = row![vis_btn, lock_btn, name_btn, up_btn, down_btn, del_btn,]
                .spacing(2)
                .align_y(Alignment::Center);

            let row_container = container(row_el).padding([2, 4]).width(Length::Fill).style(
                move |_t: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(if is_sel {
                        Color::from_rgba(0.941, 0.216, 0.031, 0.16)
                    } else {
                        theme::TRANSPARENT
                    })),
                    border: Border {
                        color: if is_sel {
                            theme::ACCENT_ORANGE
                        } else {
                            theme::TRANSPARENT
                        },
                        width: if is_sel { 1.0 } else { 0.0 },
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                },
            );

            col = col.push(row_container);
        }
    }

    if let Some(idx) = state.selected_layer_index
        && let Some(layer) = layers.get(idx)
    {
        col = col.push(layer_props_panel(layer, state));
    }

    col
}

fn layer_props_panel<'a>(
    layer: &'a crate::domain::Object,
    state: &LayerPanelState,
) -> Column<'a, Message> {
    let mut col = Column::new().spacing(8).padding([10, 14]);

    col = col.push(
        container(
            text(format!("▸ {} Properties", layer.display_name()))
                .size(11)
                .color(theme::TEXT_MUTED),
        )
        .padding([6, 0]),
    );

    col = col.push(
        row![
            button(
                text(if layer.visible {
                    "👁 Visible"
                } else {
                    "◻ Hidden"
                })
                .size(11)
            )
            .on_press(Message::from(
                layers::Message::ToggleSelectedLayerVisibility
            ))
            .style(if layer.visible {
                theme::primary_button
            } else {
                theme::secondary_button
            })
            .padding([4, 8]),
            Space::new().width(8),
            button(
                text(if layer.locked {
                    "🔒 Locked"
                } else {
                    "🔓 Unlocked"
                })
                .size(11)
            )
            .on_press(Message::from(layers::Message::ToggleSelectedLayerLock))
            .style(if layer.locked {
                theme::primary_button
            } else {
                theme::secondary_button
            })
            .padding([4, 8]),
        ]
        .spacing(0),
    );

    col = col.push(prop_label("Opacity"));
    col = col.push(
        row![
            slider(0.0..=1.0, layer.opacity, |v| Message::from(
                layers::Message::SelectedLayerOpacityChanged(v)
            ))
            .step(0.01_f32)
            .width(Length::Fill),
            text(format!("{:.0}%", layer.opacity * 100.0))
                .size(11)
                .color(theme::TEXT_MUTED)
                .width(32),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    col = col.push(prop_label("Position  (X / Y)"));
    col = col.push(
        row![
            text_input("X", state.editing_pos_x.as_str())
                .on_input(|v| Message::from(layers::Message::SelectedLayerPositionXChanged(v)))
                .size(12)
                .padding([5, 6]),
            text_input("Y", state.editing_pos_y.as_str())
                .on_input(|v| Message::from(layers::Message::SelectedLayerPositionYChanged(v)))
                .size(12)
                .padding([5, 6]),
        ]
        .spacing(6),
    );
    col = col.push(prop_label("Size  (W / H)"));
    col = col.push(
        row![
            text_input("W", state.editing_width.as_str())
                .on_input(|v| Message::from(layers::Message::SelectedLayerWidthChanged(v)))
                .size(12)
                .padding([5, 6]),
            text_input("H", state.editing_height.as_str())
                .on_input(|v| Message::from(layers::Message::SelectedLayerHeightChanged(v)))
                .size(12)
                .padding([5, 6]),
        ]
        .spacing(6),
    );

    match &layer.content {
        ObjectContent::Text {
            text: content,
            style,
            ..
        } => {
            col = col.push(prop_label("Text"));
            col = col.push(
                text_input("Layer text…", state.editing_text.as_str())
                    .on_input(|v| Message::from(layers::Message::SelectedLayerTextChanged(v)))
                    .size(12)
                    .padding([5, 6])
                    .width(Length::Fill),
            );
            col = col.push(prop_label("Font Size"));
            col = col.push(
                text_input("72", state.editing_font_size.as_str())
                    .on_input(|v| Message::from(layers::Message::SelectedLayerFontSizeChanged(v)))
                    .size(12)
                    .padding([5, 6])
                    .width(80),
            );
            col = col.push(prop_label("Colour  (R / G / B)"));
            col = col.push(color_channel_slider("R", style.color.r, |v| {
                Message::from(layers::Message::SelectedLayerTextColorR(v))
            }));
            col = col.push(color_channel_slider("G", style.color.g, |v| {
                Message::from(layers::Message::SelectedLayerTextColorG(v))
            }));
            col = col.push(color_channel_slider("B", style.color.b, |v| {
                Message::from(layers::Message::SelectedLayerTextColorB(v))
            }));
            col = col.push(
                row![
                    text("Hex").size(11).color(theme::TEXT_MUTED).width(28),
                    text_input(
                        "#RRGGBB",
                        &format!(
                            "#{:02X}{:02X}{:02X}",
                            style.color.r, style.color.g, style.color.b
                        ),
                    )
                    .on_input(|v| Message::from(typography::Message::SelectedLayerTextColorHex(v)))
                    .size(12)
                    .padding([5, 6])
                    .width(Length::Fill),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
            col = col.push(prop_label("Style"));
            col = col.push(
                row![
                    layer_toggle_btn(
                        "Bold",
                        style.bold,
                        Message::from(layers::Message::SelectedLayerTextBoldToggled)
                    ),
                    layer_toggle_btn(
                        "Italic",
                        style.italic,
                        Message::from(layers::Message::SelectedLayerTextItalicToggled)
                    ),
                    layer_toggle_btn(
                        "Shadow",
                        style.shadow,
                        Message::from(layers::Message::SelectedLayerTextShadowToggled)
                    ),
                    layer_toggle_btn(
                        "Outline",
                        style.outline,
                        Message::from(layers::Message::SelectedLayerTextOutlineToggled)
                    ),
                ]
                .spacing(4)
                .width(Length::Fill),
            );
            col = col.push(prop_label("Font Family"));
            col = col.push(
                text_input("Arial", state.editing_font_family.as_str())
                    .on_input(|v| {
                        Message::from(typography::Message::SelectedLayerFontFamilyChanged(v))
                    })
                    .on_submit(Message::from(
                        typography::Message::SelectedLayerFontFamilyChanged(
                            state.editing_font_family.clone(),
                        ),
                    ))
                    .size(12)
                    .padding([5, 6])
                    .width(Length::Fill),
            );
            col = col.push(
                row![
                    button(text("Arial").size(10))
                        .on_press(Message::from(
                            typography::Message::SelectedLayerFontFamilyChanged("Arial".into())
                        ))
                        .style(theme::tab_inactive_button)
                        .padding([3, 6]),
                    button(text("Georgia").size(10))
                        .on_press(Message::from(
                            typography::Message::SelectedLayerFontFamilyChanged("Georgia".into())
                        ))
                        .style(theme::tab_inactive_button)
                        .padding([3, 6]),
                    button(text("Impact").size(10))
                        .on_press(Message::from(
                            typography::Message::SelectedLayerFontFamilyChanged("Impact".into())
                        ))
                        .style(theme::tab_inactive_button)
                        .padding([3, 6]),
                    button(text("Courier").size(10))
                        .on_press(Message::from(
                            typography::Message::SelectedLayerFontFamilyChanged(
                                "Courier New".into(),
                            )
                        ))
                        .style(theme::tab_inactive_button)
                        .padding([3, 6]),
                ]
                .spacing(4),
            );
            col = col.push(prop_label("Line Height / Letter Spacing"));
            col = col.push(
                row![
                    text("LH").size(11).color(theme::TEXT_MUTED).width(20),
                    text_input("1.2", state.editing_line_height.as_str())
                        .on_input(|v| Message::from(
                            typography::Message::SelectedLayerLineHeightChanged(v)
                        ))
                        .size(12)
                        .padding([5, 6])
                        .width(64),
                    text("LS").size(11).color(theme::TEXT_MUTED).width(20),
                    text_input("0", state.editing_letter_spacing.as_str())
                        .on_input(|v| Message::from(
                            typography::Message::SelectedLayerLetterSpacingChanged(v)
                        ))
                        .size(12)
                        .padding([5, 6])
                        .width(64),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
            col = col.push(prop_label("Text Transform"));
            col = col.push(
                row![
                    layer_toggle_btn(
                        "None",
                        style.text_transform == TextTransform::None,
                        Message::from(typography::Message::SelectedLayerTextTransform(
                            TextTransform::None
                        )),
                    ),
                    layer_toggle_btn(
                        "UPPER",
                        style.text_transform == TextTransform::Uppercase,
                        Message::from(typography::Message::SelectedLayerTextTransform(
                            TextTransform::Uppercase
                        )),
                    ),
                    layer_toggle_btn(
                        "lower",
                        style.text_transform == TextTransform::Lowercase,
                        Message::from(typography::Message::SelectedLayerTextTransform(
                            TextTransform::Lowercase
                        )),
                    ),
                    layer_toggle_btn(
                        "Title",
                        style.text_transform == TextTransform::Capitalize,
                        Message::from(typography::Message::SelectedLayerTextTransform(
                            TextTransform::Capitalize
                        )),
                    ),
                ]
                .spacing(4)
                .width(Length::Fill),
            );
            col = col.push(prop_label("Glow"));
            col = col.push(
                row![
                    layer_toggle_btn(
                        if style.glow_enabled { "On" } else { "Off" },
                        style.glow_enabled,
                        Message::from(typography::Message::SelectedLayerGlowToggled),
                    ),
                    text(" Radius").size(11).color(theme::TEXT_MUTED),
                    text_input("8", state.editing_glow_radius.as_str())
                        .on_input(|v| Message::from(
                            typography::Message::SelectedLayerGlowRadiusChanged(v)
                        ))
                        .size(12)
                        .padding([5, 6])
                        .width(52),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
            if style.glow_enabled {
                col = col.push(prop_label("Glow Colour  (R / G / B)"));
                col = col.push(color_channel_slider("R", style.glow_color.r, |v| {
                    Message::from(typography::Message::SelectedLayerGlowColorR(v))
                }));
                col = col.push(color_channel_slider("G", style.glow_color.g, |v| {
                    Message::from(typography::Message::SelectedLayerGlowColorG(v))
                }));
                col = col.push(color_channel_slider("B", style.glow_color.b, |v| {
                    Message::from(typography::Message::SelectedLayerGlowColorB(v))
                }));
            }
            col = col.push(prop_label("Text Stroke"));
            col = col.push(
                row![
                    text("W").size(11).color(theme::TEXT_MUTED).width(14),
                    text_input("0", state.editing_text_stroke_width.as_str())
                        .on_input(|v| Message::from(
                            typography::Message::SelectedLayerTextStrokeWidthChanged(v)
                        ))
                        .size(12)
                        .padding([5, 6])
                        .width(64),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
            if style.text_stroke_width > 0.0 {
                col = col.push(prop_label("Stroke Colour  (R / G / B)"));
                col = col.push(color_channel_slider("R", style.text_stroke_color.r, |v| {
                    Message::from(typography::Message::SelectedLayerTextStrokeColorR(v))
                }));
                col = col.push(color_channel_slider("G", style.text_stroke_color.g, |v| {
                    Message::from(typography::Message::SelectedLayerTextStrokeColorG(v))
                }));
                col = col.push(color_channel_slider("B", style.text_stroke_color.b, |v| {
                    Message::from(typography::Message::SelectedLayerTextStrokeColorB(v))
                }));
            }
            let _ = content;
        }
        ObjectContent::Shape {
            fill, stroke_width, ..
        } => {
            col = col.push(prop_label("Fill Colour  (R / G / B / A)"));
            col = col.push(color_channel_slider("R", fill.r, |v| {
                Message::from(layers::Message::SelectedLayerShapeFillR(v))
            }));
            col = col.push(color_channel_slider("G", fill.g, |v| {
                Message::from(layers::Message::SelectedLayerShapeFillG(v))
            }));
            col = col.push(color_channel_slider("B", fill.b, |v| {
                Message::from(layers::Message::SelectedLayerShapeFillB(v))
            }));
            col = col.push(color_channel_slider("A", fill.a, |v| {
                Message::from(layers::Message::SelectedLayerShapeFillA(v))
            }));
            col = col.push(prop_label("Stroke Width"));
            col = col.push(
                text_input("0", state.editing_stroke_width.as_str())
                    .on_input(|v| {
                        Message::from(layers::Message::SelectedLayerShapeStrokeWidthChanged(v))
                    })
                    .size(12)
                    .padding([5, 6])
                    .width(80),
            );
            let _ = stroke_width;
        }
        ObjectContent::Image { path, .. } => {
            col = col.push(
                container(
                    text(if path.is_empty() {
                        "No image set"
                    } else {
                        path.as_str()
                    })
                    .size(11)
                    .color(theme::TEXT_MUTED),
                )
                .padding([4, 0]),
            );
        }
        ObjectContent::Video { path, .. } => {
            col = col.push(
                container(
                    text(if path.is_empty() {
                        "No video set"
                    } else {
                        path.as_str()
                    })
                    .size(11)
                    .color(theme::TEXT_MUTED),
                )
                .padding([4, 0]),
            );
        }
    }

    col
}

fn prop_label<'a>(label: &'a str) -> Element<'a, Message> {
    section_label(label)
}

fn layer_toggle_btn<'a>(label: &'a str, active: bool, msg: Message) -> Element<'a, Message> {
    compact_toggle_btn(label, active, msg)
}
