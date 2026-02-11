use iced::Size;
use openpresenter::{Result, app::App, ui::MainWindow};
use std::sync::Arc;

fn main() -> Result<()> {
    openpresenter::ndi::initialize()?;

    let app = App::new()?;
    println!("✓ Database initialized at: {:?}", app.db().path());

    let db = Arc::new(app.into_db());

    iced::application(
        move || MainWindow::new(db.clone()),
        MainWindow::update,
        MainWindow::view,
    )
    .window_size(Size::new(1200.0, 800.0))
    .run()?;

    openpresenter::ndi::destroy();

    Ok(())
}
