use vt100::common::WindowAction;
use std::sync::mpsc::Sender;

pub trait TerminalWindow {
    fn on_window_action(&self, action: WindowAction); 
}

impl TerminalWindow for Sender<WindowAction> {
    fn on_window_action(&self, action: WindowAction) {
        self.send(action).expect("Channel should be able to send window action");
    }
}
