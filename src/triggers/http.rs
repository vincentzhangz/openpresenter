use super::Action;
use axum::{
    Router,
    extract::{Path, State},
    response::Json,
    routing::{get, post},
};
use serde_json::{Value, json};
use tokio::sync::{mpsc::Sender, oneshot};

/// One HTTP route the remote control surface serves.
///
/// The triggers panel lists these so the operator can configure a controller
/// from the same list the server serves.
#[derive(Debug, Clone, Copy)]
pub struct RouteInfo {
    pub method: &'static str,
    pub path: &'static str,
    pub what: &'static str,
}

/// The HTTP routes served by [`run_server`].
pub const ROUTES: &[RouteInfo] = &[
    RouteInfo {
        method: "GET",
        path: "/api/status",
        what: "Health check",
    },
    RouteInfo {
        method: "POST",
        path: "/api/slides/next",
        what: "Next slide",
    },
    RouteInfo {
        method: "POST",
        path: "/api/slides/prev",
        what: "Previous slide",
    },
    RouteInfo {
        method: "POST",
        path: "/api/slides/goto/{index}",
        what: "Go to slide by index",
    },
    RouteInfo {
        method: "POST",
        path: "/api/black/{on}",
        what: "Black screen on/off",
    },
    RouteInfo {
        method: "POST",
        path: "/api/clear",
        what: "Clear output",
    },
    RouteInfo {
        method: "POST",
        path: "/api/timer/start",
        what: "Start timer",
    },
    RouteInfo {
        method: "POST",
        path: "/api/timer/stop",
        what: "Stop timer",
    },
    RouteInfo {
        method: "POST",
        path: "/api/timer/reset",
        what: "Reset timer",
    },
];

pub async fn run_server(
    port: u16,
    tx: Sender<Action>,
    shutdown: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/slides/next", post(next_slide))
        .route("/api/slides/prev", post(prev_slide))
        .route("/api/slides/goto/{index}", post(goto_slide))
        .route("/api/black/{on}", post(black_screen))
        .route("/api/clear", post(clear_output))
        .route("/api/timer/start", post(timer_start))
        .route("/api/timer/stop", post(timer_stop))
        .route("/api/timer/reset", post(timer_reset))
        .with_state(tx);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown.await;
        })
        .await?;
    Ok(())
}

async fn status() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn next_slide(State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::NextSlide).await;
    Json(json!({"action": "next"}))
}

async fn prev_slide(State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::PrevSlide).await;
    Json(json!({"action": "prev"}))
}

async fn goto_slide(Path(index): Path<usize>, State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::GotoSlide(index)).await;
    Json(json!({"action": "goto", "index": index}))
}

fn parse_on_param(s: &str) -> bool {
    matches!(s, "true" | "1" | "on")
}

async fn black_screen(Path(on): Path<String>, State(tx): State<Sender<Action>>) -> Json<Value> {
    let black = parse_on_param(&on);
    let _ = tx.send(Action::BlackScreen(black)).await;
    Json(json!({"action": "black", "value": black}))
}

async fn clear_output(State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::ClearOutput).await;
    Json(json!({"action": "clear"}))
}

async fn timer_start(State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::StartTimer).await;
    Json(json!({"action": "timer_start"}))
}

async fn timer_stop(State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::StopTimer).await;
    Json(json!({"action": "timer_stop"}))
}

async fn timer_reset(State(tx): State<Sender<Action>>) -> Json<Value> {
    let _ = tx.send(Action::ResetTimer).await;
    Json(json!({"action": "timer_reset"}))
}

#[cfg(test)]
mod tests {
    use super::parse_on_param;

    #[test]
    fn parse_on_recognises_true_values() {
        assert!(parse_on_param("true"));
        assert!(parse_on_param("1"));
        assert!(parse_on_param("on"));
    }

    #[test]
    fn parse_on_rejects_false_values() {
        assert!(!parse_on_param("false"));
        assert!(!parse_on_param("0"));
        assert!(!parse_on_param("off"));
        assert!(!parse_on_param(""));
        assert!(!parse_on_param("yes"));
        assert!(!parse_on_param("TRUE"));
    }
}
