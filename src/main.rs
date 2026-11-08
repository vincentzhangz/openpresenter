use gpui_kit::AppContext;
use openpresenter::{Result, app::App, ui::MainWindow};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<()> {
    let startup = Instant::now();

    // Without this every log::error!/warn! in the app writes to nowhere.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    openpresenter::ndi::initialize()?;

    let app = App::new()?;
    log::info!("database initialized at {:?}", app.db().path());

    let db = Arc::new(app.into_db());
    log::info!("app initialized in {}ms", startup.elapsed().as_millis());

    let config = openpresenter::config::Config::load();
    let db_clone = db.clone();
    let config_clone = config.clone();

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        openpresenter::ui::theme::install(cx);
        openpresenter::ui::actions::bind_keys(cx);
        cx.set_menus(openpresenter::ui::menus::build(
            &openpresenter::ui::menus::MenuState::default(),
        ));
        let db = db_clone.clone();
        let config = config_clone.clone();
        let options = gpui_kit::WindowOptions {
            window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                origin: Default::default(),
                size: gpui_kit::size(gpui_kit::px(1400.0), gpui_kit::px(900.0)),
            })),
            focus: true,
            show: true,
            ..Default::default()
        };
        let _ = gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| MainWindow::new(db.clone(), config.clone(), window, cx))
        });
    });

    openpresenter::ndi::destroy();

    Ok(())
}
