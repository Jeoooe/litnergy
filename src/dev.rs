#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub type FakeDevice = linux::FakeDevice;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub type FakeDevice = macos::FakeDevice;

pub enum KeyState {
    UP = 0,
    DOWN = 1,
}

pub enum ButtonType {
    Left = 0,
    Middle = 1,
    Right = 2,
    Extra = 7,
}
