use crate::db::Database;
use crate::slides::Presentation;
use iced::widget::{Column, Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Task};
use std::sync::Arc;

pub struct MainWindow {
    search_query: String,
    show_new_presentation_dialog: bool,
    new_presentation_name: String,
    presentations: Vec<Presentation>,
    selected_presentation: Option<String>,
    db: Arc<Database>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchQueryChanged(String),
    NewPresentationClicked,
    CreatePresentation,
    CancelNewPresentation,
    NewPresentationNameChanged(String),
    SelectPresentation(String),
    Quit,
}

impl MainWindow {
    pub fn new(db: Arc<Database>) -> (Self, Task<Message>) {
        let mut window = Self {
            search_query: String::new(),
            show_new_presentation_dialog: false,
            new_presentation_name: String::new(),
            presentations: Vec::new(),
            selected_presentation: None,
            db,
        };
        window.load_presentations();
        (window, Task::none())
    }

    fn load_presentations(&mut self) {
        if let Ok(conn) = self.db.connection().lock() {
            let stmt = conn.prepare(
                "SELECT id, name, created_at, updated_at FROM presentations ORDER BY updated_at DESC"
            ).ok();

            if let Some(mut stmt) = stmt {
                let presentations_iter = stmt
                    .query_map([], |row| {
                        use chrono::TimeZone;

                        let created_timestamp: i64 = row.get(2)?;
                        let updated_timestamp: i64 = row.get(3)?;

                        Ok(Presentation {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            slides: Vec::new(),
                            created_at: chrono::Utc.timestamp_opt(created_timestamp, 0).unwrap(),
                            updated_at: chrono::Utc.timestamp_opt(updated_timestamp, 0).unwrap(),
                        })
                    })
                    .ok();

                if let Some(iter) = presentations_iter {
                    self.presentations = iter.filter_map(|r| r.ok()).collect();
                }
            }
        }
    }

    fn create_presentation(&mut self, name: String) {
        use chrono::Utc;
        use uuid::Uuid;

        let presentation = Presentation {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            slides: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let success = {
            if let Ok(conn) = self.db.connection().lock() {
                let result = conn.execute(
                    "INSERT INTO presentations (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    (&presentation.id, &presentation.name, presentation.created_at.timestamp(), presentation.updated_at.timestamp()),
                );
                result.is_ok()
            } else {
                false
            }
        };

        if success {
            println!("✓ Created presentation: {}", name);
            self.load_presentations();
        } else {
            eprintln!("✗ Failed to create presentation");
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchQueryChanged(query) => {
                self.search_query = query;
            }
            Message::NewPresentationClicked => {
                self.show_new_presentation_dialog = true;
            }
            Message::CreatePresentation => {
                if !self.new_presentation_name.trim().is_empty() {
                    self.create_presentation(self.new_presentation_name.clone());
                    self.new_presentation_name.clear();
                    self.show_new_presentation_dialog = false;
                }
            }
            Message::CancelNewPresentation => {
                self.new_presentation_name.clear();
                self.show_new_presentation_dialog = false;
            }
            Message::NewPresentationNameChanged(name) => {
                self.new_presentation_name = name;
            }
            Message::SelectPresentation(id) => {
                self.selected_presentation = Some(id.clone());
                println!(
                    "Selected presentation: {}",
                    self.presentations
                        .iter()
                        .find(|p| p.id == id)
                        .map(|p| p.name.as_str())
                        .unwrap_or("unknown")
                );
            }
            Message::Quit => {
                std::process::exit(0);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = self.presentations_sidebar();
        let main_content = self.main_content();

        let content = row![sidebar, main_content]
            .width(Length::Fill)
            .height(Length::Fill);

        let content_with_dialog = if self.show_new_presentation_dialog {
            column![content, self.new_presentation_dialog()]
        } else {
            column![content]
        };

        container(content_with_dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn presentations_sidebar(&self) -> Element<'_, Message> {
        let mut sidebar_content = Column::new().padding(10).spacing(10).width(250);

        sidebar_content = sidebar_content.push(text("Presentations").size(24));

        let mut presentations_list = Column::new().spacing(5);

        if self.presentations.is_empty() {
            presentations_list = presentations_list.push(text("No presentations yet").size(14));
        } else {
            for presentation in &self.presentations {
                let is_selected = self.selected_presentation.as_ref() == Some(&presentation.id);
                let btn = button(text(&presentation.name).size(14))
                    .width(Length::Fill)
                    .on_press(Message::SelectPresentation(presentation.id.clone()));

                let btn = if is_selected {
                    btn.style(button::primary)
                } else {
                    btn.style(button::secondary)
                };

                presentations_list = presentations_list.push(btn);
            }
        }

        sidebar_content = sidebar_content.push(scrollable(presentations_list).height(Length::Fill));

        sidebar_content = sidebar_content.push(
            button(text("➕ New Presentation").size(14))
                .width(Length::Fill)
                .on_press(Message::NewPresentationClicked),
        );

        container(sidebar_content)
            .width(250)
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }

    fn main_content(&self) -> Element<'_, Message> {
        let header = column![
            text("OpenPresenter").size(32),
            Space::new().height(10),
            row![
                text("Search:").size(16),
                text_input("Search...", &self.search_query)
                    .on_input(Message::SearchQueryChanged)
                    .width(300)
            ]
            .spacing(10),
        ]
        .padding(20);

        let welcome_section = column![
            text("Welcome to OpenPresenter").size(28),
            Space::new().height(20),
            text("Create a new presentation or open an existing one to get started.").size(16),
            Space::new().height(30),
            button(text("Create New Presentation").size(16))
                .on_press(Message::NewPresentationClicked)
                .padding([12, 24]),
            Space::new().height(12),
            button(text("Open Presentation").size(16)).padding([12, 24]),
        ]
        .align_x(iced::Alignment::Center)
        .width(Length::Fill);

        let content = column![
            header,
            container(welcome_section)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn new_presentation_dialog(&self) -> Element<'_, Message> {
        let dialog_content = column![
            text("New Presentation").size(20),
            Space::new().height(10),
            text("Enter a name for your presentation:").size(14),
            Space::new().height(10),
            row![
                text("Name:").size(14),
                text_input("Presentation name", &self.new_presentation_name)
                    .on_input(Message::NewPresentationNameChanged)
                    .width(300)
            ]
            .spacing(10),
            Space::new().height(20),
            row![
                button(text("Create").size(14))
                    .on_press(Message::CreatePresentation)
                    .padding(10)
                    .style(button::primary),
                button(text("Cancel").size(14))
                    .on_press(Message::CancelNewPresentation)
                    .padding(10)
                    .style(button::secondary),
            ]
            .spacing(10)
        ]
        .padding(20)
        .spacing(10)
        .align_x(iced::Alignment::Center);

        container(dialog_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.0, 0.0, 0.0, 0.7,
                ))),
                ..Default::default()
            })
            .into()
    }
}
