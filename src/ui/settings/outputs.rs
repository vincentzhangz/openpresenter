use crate::output::{NamedOutput, OutputType};
use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, Space, button, column, container, row, scrollable, text, text_input, toggler,
    },
};

pub fn outputs_panel<'a>(
    outputs: impl Iterator<Item = &'a NamedOutput>,
    new_output_label: &'a str,
    new_output_ndi_name: &'a str,
) -> Element<'a, Message> {
    let header = row![text("Outputs").size(18.0), Space::new().width(Length::Fill),]
        .align_y(Alignment::Center);

    let mut output_rows: Column<'a, Message> = column![].spacing(4);
    for output in outputs {
        output_rows = output_rows.push(output_row(output));
    }

    let add_form = add_output_form(new_output_label, new_output_ndi_name);

    container(
        column![
            header,
            scrollable(output_rows).height(Length::Fill),
            add_form,
        ]
        .spacing(8)
        .padding(12),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn output_row<'a>(output: &'a NamedOutput) -> Element<'a, Message> {
    let type_label = match &output.output_type {
        OutputType::Window => "Window".to_string(),
        OutputType::Ndi { stream_name } => format!("NDI: {stream_name}"),
    };

    let resolution = format!("{}×{}", output.width, output.height);

    let content_label = output.content.to_string();

    let content_btn = button(text(content_label).size(11.0))
        .on_press(Message::OutputCycleContent(output.id.clone()))
        .padding([3, 8])
        .style(theme::ghost_button);

    let res_720 = button(text("720p").size(11.0))
        .on_press(Message::OutputSetResolution(output.id.clone(), 1280, 720))
        .padding([3, 6])
        .style(theme::ghost_button);
    let res_1080 = button(text("1080p").size(11.0))
        .on_press(Message::OutputSetResolution(output.id.clone(), 1920, 1080))
        .padding([3, 6])
        .style(theme::ghost_button);

    let active_toggle =
        toggler(output.active).on_toggle(move |v| Message::OutputSetActive(output.id.clone(), v));

    let delete_btn = button(text("X").size(11.0))
        .on_press(Message::OutputRemove(output.id.clone()))
        .padding([3, 6])
        .style(theme::danger_button);

    let row = row![
        active_toggle,
        text(&output.label).size(13.0).width(120),
        text(type_label).size(11.0).width(100),
        text(resolution).size(11.0).width(70),
        content_btn,
        Space::new().width(Length::Fill),
        res_720,
        res_1080,
        delete_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(4)
    .padding([4, 0]);

    container(row)
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn add_output_form<'a>(label: &'a str, ndi_name: &'a str) -> Element<'a, Message> {
    let label_input = text_input("Output label…", label)
        .on_input(Message::OutputNewLabelChanged)
        .padding([4, 8])
        .width(160);

    let ndi_input = text_input("NDI stream name (optional)…", ndi_name)
        .on_input(Message::OutputNewNdiNameChanged)
        .padding([4, 8])
        .width(200);

    let add_window_btn = button(text("+ Window Output").size(12.0))
        .on_press(Message::OutputAddWindow)
        .padding([5, 10])
        .style(theme::primary_button);

    let add_ndi_btn = button(text("+ NDI Output").size(12.0))
        .on_press(Message::OutputAddNdi)
        .padding([5, 10])
        .style(theme::ghost_button);

    row![label_input, ndi_input, add_window_btn, add_ndi_btn,]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}
