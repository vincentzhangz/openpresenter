use crate::ui::messages::Message;
use crate::ui::theme;
use iced_font_awesome::fa_icon_solid;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, container, row, slider, text},
};

pub fn audio_bar<'a>(
    track_name: Option<&'a str>,
    is_playing: bool,
    volume: f32,
    looping: bool,
) -> Element<'a, Message> {
    let track_label = track_name
        .map(|t| text(t).size(12))
        .unwrap_or_else(|| text("No track loaded").size(12));

    let btn_size = 11.0f32;
    let open_btn = button(text("Open…").size(btn_size))
        .on_press(Message::AudioPickFile)
        .padding([5u16, 10u16])
        .style(theme::ghost_button);

    let play_pause = if is_playing {
        button(fa_icon_solid("pause").size(btn_size)).on_press(Message::AudioPause)
    } else {
        button(fa_icon_solid("play").size(btn_size)).on_press(Message::AudioPlay)
    };
    let play_pause = play_pause
        .padding([5u16, 10u16])
        .style(theme::primary_button);

    let stop_btn = button(fa_icon_solid("stop").size(btn_size))
        .on_press(Message::AudioStop)
        .padding([5u16, 10u16])
        .style(theme::ghost_button);

    let loop_btn = button(fa_icon_solid("repeat").size(btn_size))
        .on_press(Message::AudioToggleLoop)
        .padding([5u16, 10u16])
        .style(if looping { theme::primary_button } else { theme::ghost_button });

    let vol_slider = slider(0.0..=1.0, volume, Message::AudioVolumeChanged)
        .step(0.01)
        .width(80);

    let vol_label = text(format!("Vol {:.0}%", volume * 100.0)).size(11);

    container(
        row![
            open_btn,
            Space::new().width(8),
            track_label,
            Space::new().width(Length::Fill),
            stop_btn,
            play_pause,
            loop_btn,
            Space::new().width(8),
            vol_slider,
            vol_label,
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([6u16, 12u16]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}
