use iced::{Size, Task, window};
use openpresenter::{Result, app::App, ui::MainWindow};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<()> {
    let startup = Instant::now();

    openpresenter::ndi::initialize()?;

    let app = App::new()?;
    println!("✓ Database initialized at: {:?}", app.db().path());

    let db = Arc::new(app.into_db());
    println!("✓ App initialized in {}ms", startup.elapsed().as_millis());

    let db_clone = db.clone();
    iced::daemon(
        move || {
            let (main_id, open_task) = window::open(window::Settings {
                size: Size::new(1200.0, 800.0),
                ..Default::default()
            });
            let (state, init_task) = MainWindow::new(db_clone.clone(), main_id);
            (
                state,
                Task::batch([
                    open_task.map(|_| openpresenter::ui::Message::OutputWindowOpened),
                    init_task,
                ]),
            )
        },
        MainWindow::update,
        MainWindow::view_for_window,
    )
    .title(|state: &MainWindow, id| state.title(id))
    .theme(|_state: &MainWindow, _id| iced::Theme::Dark)
    .subscription(MainWindow::subscription)
    .run()?;

    openpresenter::ndi::destroy();

    Ok(())
}
