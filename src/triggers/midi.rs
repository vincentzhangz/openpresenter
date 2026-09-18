use crate::domain::Action;
use midir::{MidiInput, MidiInputConnection};
use tokio::sync::mpsc::Sender;

/// Discovered MIDI input device port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiPortInfo {
    pub index: usize,
    pub name: String,
}

/// Lists available MIDI input device ports on the host system.
pub fn list_midi_inputs() -> Vec<MidiPortInfo> {
    let Ok(midi_in) = MidiInput::new("OpenPresenter") else {
        return Vec::new();
    };
    let ports = midi_in.ports();
    ports
        .into_iter()
        .enumerate()
        .map(|(idx, p)| {
            let name = midi_in
                .port_name(&p)
                .unwrap_or_else(|_| format!("Port {idx}"));
            MidiPortInfo { index: idx, name }
        })
        .collect()
}

/// Parse raw MIDI message bytes into an `Action`.
///
/// Standard live production default mappings:
/// - Note On 60 (Middle C): `Action::NextSlide`
/// - Note On 59 (B3): `Action::PrevSlide`
/// - Note On 62 (D4): `Action::BlackScreen`
/// - Note On 64 (E4): `Action::ClearOutput`
/// - CC 64 (Sustain/Damper pedal down > 63): `Action::NextSlide`
pub fn parse_midi_message(bytes: &[u8]) -> Option<Action> {
    if bytes.is_empty() {
        return None;
    }
    let status = bytes[0] & 0xF0;
    match status {
        0x90 if bytes.len() >= 3 => {
            // Note On
            let note = bytes[1];
            let velocity = bytes[2];
            if velocity > 0 {
                return match note {
                    60 => Some(Action::NextSlide),
                    59 => Some(Action::PrevSlide),
                    62 => Some(Action::BlackScreen(true)),
                    63 => Some(Action::BlackScreen(false)),
                    64 => Some(Action::ClearOutput),
                    _ => None,
                };
            }
        }
        0xB0 if bytes.len() >= 3 => {
            // Control Change (CC)
            let cc_num = bytes[1];
            let value = bytes[2];
            if cc_num == 64 && value > 63 {
                return Some(Action::NextSlide);
            }
        }
        _ => {}
    }
    None
}

/// Active MIDI input connection listener.
pub struct MidiListener {
    _connection: MidiInputConnection<()>,
}

impl MidiListener {
    /// Start listening on the specified MIDI input port index.
    pub fn start(port_index: usize, sender: Sender<Action>) -> Result<Self, anyhow::Error> {
        let midi_in = MidiInput::new("OpenPresenter")?;
        let ports = midi_in.ports();
        let port = ports
            .get(port_index)
            .ok_or_else(|| anyhow::anyhow!("MIDI port index {port_index} out of range"))?;

        let conn = midi_in
            .connect(
                port,
                "openpresenter-input",
                move |_timestamp, bytes, _| {
                    if let Some(action) = parse_midi_message(bytes) {
                        let _ = sender.try_send(action);
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("Failed to connect to MIDI device: {e}"))?;

        Ok(Self { _connection: conn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_midi_note_on() {
        // Note 60 on, velocity 100
        assert_eq!(
            parse_midi_message(&[0x90, 60, 100]),
            Some(Action::NextSlide)
        );
        // Note 59 on, velocity 80
        assert_eq!(parse_midi_message(&[0x90, 59, 80]), Some(Action::PrevSlide));
        // Note 62 on, velocity 127
        assert_eq!(
            parse_midi_message(&[0x90, 62, 127]),
            Some(Action::BlackScreen(true))
        );
        // Note 64 on, velocity 64
        assert_eq!(
            parse_midi_message(&[0x90, 64, 64]),
            Some(Action::ClearOutput)
        );
        // Note On with velocity 0 is treated as Note Off
        assert_eq!(parse_midi_message(&[0x90, 60, 0]), None);
        // Unmapped note
        assert_eq!(parse_midi_message(&[0x90, 72, 100]), None);
    }

    #[test]
    fn test_parse_midi_cc() {
        // CC 64 (sustain) value 127
        assert_eq!(
            parse_midi_message(&[0xB0, 64, 127]),
            Some(Action::NextSlide)
        );
        // CC 64 value 0 (pedal release)
        assert_eq!(parse_midi_message(&[0xB0, 64, 0]), None);
        // Other CC
        assert_eq!(parse_midi_message(&[0xB0, 7, 100]), None);
    }
}
