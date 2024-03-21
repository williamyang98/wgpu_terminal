use vt100::common::WindowAction;

#[derive(Clone,Debug)]
pub enum AppEvent {
    WindowAction(WindowAction),
}
