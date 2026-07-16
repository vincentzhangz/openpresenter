use super::Action;
use rosc::{OscPacket, OscType};
use tokio::net::UdpSocket;

pub async fn run_listener(port: u16, tx: tokio::sync::mpsc::Sender<Action>) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).await?;
    let mut buf = [0u8; 65536];
    loop {
        let (n, _) = socket.recv_from(&mut buf).await?;
        match rosc::decoder::decode_udp(&buf[..n]) {
            Ok((_, OscPacket::Message(msg))) => {
                if let Some(action) = dispatch_osc(&msg) {
                    let _ = tx.send(action).await;
                }
            }
            Ok((_, OscPacket::Bundle(bundle))) => {
                for item in bundle.content {
                    if let OscPacket::Message(msg) = item
                        && let Some(action) = dispatch_osc(&msg)
                    {
                        let _ = tx.send(action).await;
                    }
                }
            }
            Err(e) => eprintln!("[osc] decode error: {e}"),
        }
    }
}

fn dispatch_osc(msg: &rosc::OscMessage) -> Option<Action> {
    match msg.addr.as_str() {
        "/slide/next" => Some(Action::NextSlide),
        "/slide/prev" => Some(Action::PrevSlide),
        "/slide/goto" => {
            let idx = msg
                .args
                .first()
                .and_then(|a| match a {
                    OscType::Int(n) => Some(*n as usize),
                    OscType::Float(f) => Some(*f as usize),
                    _ => None,
                })
                .unwrap_or(0);
            Some(Action::GotoSlide(idx))
        }
        "/black" => {
            let on = msg
                .args
                .first()
                .map(|a| match a {
                    OscType::Int(n) => *n != 0,
                    OscType::Bool(b) => *b,
                    _ => false,
                })
                .unwrap_or(false);
            Some(Action::BlackScreen(on))
        }
        "/clear" => Some(Action::ClearOutput),
        "/timer/start" => Some(Action::StartTimer),
        "/timer/stop" => Some(Action::StopTimer),
        "/timer/reset" => Some(Action::ResetTimer),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::{OscMessage, OscType};

    fn msg(addr: &str, args: Vec<OscType>) -> OscMessage {
        OscMessage {
            addr: addr.to_string(),
            args,
        }
    }

    #[test]
    fn dispatch_next_slide() {
        assert!(matches!(
            dispatch_osc(&msg("/slide/next", vec![])),
            Some(Action::NextSlide)
        ));
    }

    #[test]
    fn dispatch_prev_slide() {
        assert!(matches!(
            dispatch_osc(&msg("/slide/prev", vec![])),
            Some(Action::PrevSlide)
        ));
    }

    #[test]
    fn dispatch_goto_slide_int_arg() {
        let action = dispatch_osc(&msg("/slide/goto", vec![OscType::Int(3)]));
        assert!(matches!(action, Some(Action::GotoSlide(3))));
    }

    #[test]
    fn dispatch_goto_slide_float_arg() {
        let action = dispatch_osc(&msg("/slide/goto", vec![OscType::Float(5.0)]));
        assert!(matches!(action, Some(Action::GotoSlide(5))));
    }

    #[test]
    fn dispatch_goto_slide_no_arg_defaults_to_zero() {
        let action = dispatch_osc(&msg("/slide/goto", vec![]));
        assert!(matches!(action, Some(Action::GotoSlide(0))));
    }

    #[test]
    fn dispatch_black_int_nonzero_is_true() {
        let action = dispatch_osc(&msg("/black", vec![OscType::Int(1)]));
        assert!(matches!(action, Some(Action::BlackScreen(true))));
    }

    #[test]
    fn dispatch_black_int_zero_is_false() {
        let action = dispatch_osc(&msg("/black", vec![OscType::Int(0)]));
        assert!(matches!(action, Some(Action::BlackScreen(false))));
    }

    #[test]
    fn dispatch_black_bool_true() {
        let action = dispatch_osc(&msg("/black", vec![OscType::Bool(true)]));
        assert!(matches!(action, Some(Action::BlackScreen(true))));
    }

    #[test]
    fn dispatch_black_no_arg_defaults_false() {
        let action = dispatch_osc(&msg("/black", vec![]));
        assert!(matches!(action, Some(Action::BlackScreen(false))));
    }

    #[test]
    fn dispatch_clear() {
        assert!(matches!(
            dispatch_osc(&msg("/clear", vec![])),
            Some(Action::ClearOutput)
        ));
    }

    #[test]
    fn dispatch_timer_start() {
        assert!(matches!(
            dispatch_osc(&msg("/timer/start", vec![])),
            Some(Action::StartTimer)
        ));
    }

    #[test]
    fn dispatch_timer_stop() {
        assert!(matches!(
            dispatch_osc(&msg("/timer/stop", vec![])),
            Some(Action::StopTimer)
        ));
    }

    #[test]
    fn dispatch_timer_reset() {
        assert!(matches!(
            dispatch_osc(&msg("/timer/reset", vec![])),
            Some(Action::ResetTimer)
        ));
    }

    #[test]
    fn dispatch_unknown_addr_returns_none() {
        assert!(dispatch_osc(&msg("/unknown/addr", vec![])).is_none());
    }
}
