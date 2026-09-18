pub mod automation;
pub mod http;
pub mod midi;
pub mod osc;

pub use crate::domain::Action;

pub struct TriggerManager {
    sender: tokio::sync::mpsc::Sender<Action>,

    http_handle: Option<tokio::task::JoinHandle<()>>,
    http_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub http_port: u16,
    pub http_running: bool,

    osc_handle: Option<tokio::task::JoinHandle<()>>,
    pub osc_port: u16,
    pub osc_running: bool,

    midi_listener: Option<midi::MidiListener>,
    pub midi_port: Option<usize>,
    pub midi_running: bool,

    pub macros: Vec<automation::Macro>,
}

impl Default for TriggerManager {
    fn default() -> Self {
        let (sender, _discarded) = tokio::sync::mpsc::channel(64);
        Self {
            sender,
            http_handle: None,
            http_shutdown: None,
            http_port: 9090,
            http_running: false,
            osc_handle: None,
            osc_port: 9000,
            osc_running: false,
            midi_listener: None,
            midi_port: None,
            midi_running: false,
            macros: Vec::new(),
        }
    }
}

impl TriggerManager {
    pub fn subscribe(&mut self) -> tokio::sync::mpsc::Receiver<Action> {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        self.sender = tx;
        rx
    }

    pub fn sender(&self) -> tokio::sync::mpsc::Sender<Action> {
        self.sender.clone()
    }

    pub fn start_http(&mut self) {
        if self.http_running {
            return;
        }
        let sender = self.sender.clone();
        let port = self.http_port;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            if let Err(e) = http::run_server(port, sender, shutdown_rx).await {
                eprintln!("[triggers/http] server error: {e}");
            }
        });
        self.http_handle = Some(handle);
        self.http_shutdown = Some(shutdown_tx);
        self.http_running = true;
    }

    pub fn stop_http(&mut self) {
        if let Some(tx) = self.http_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.http_handle.take() {
            h.abort();
        }
        self.http_running = false;
    }

    pub fn start_osc(&mut self) {
        if self.osc_running {
            return;
        }
        let sender = self.sender.clone();
        let port = self.osc_port;
        let handle = tokio::spawn(async move {
            if let Err(e) = osc::run_listener(port, sender).await {
                eprintln!("[triggers/osc] listener error: {e}");
            }
        });
        self.osc_handle = Some(handle);
        self.osc_running = true;
    }

    pub fn stop_osc(&mut self) {
        if let Some(h) = self.osc_handle.take() {
            h.abort();
        }
        self.osc_running = false;
    }

    pub fn start_midi(&mut self, port_index: usize) -> Result<(), anyhow::Error> {
        self.stop_midi();
        let listener = midi::MidiListener::start(port_index, self.sender.clone())?;
        self.midi_listener = Some(listener);
        self.midi_port = Some(port_index);
        self.midi_running = true;
        Ok(())
    }

    pub fn stop_midi(&mut self) {
        self.midi_listener = None;
        self.midi_port = None;
        self.midi_running = false;
    }
}

impl Drop for TriggerManager {
    fn drop(&mut self) {
        self.stop_http();
        self.stop_osc();
        self.stop_midi();
    }
}
