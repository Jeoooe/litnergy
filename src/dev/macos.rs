use std::{collections::HashSet, io::ErrorKind, thread, time::Duration};

use core_graphics::{
    event::{
        CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, ScrollEventUnit,
    },
    event_source::{CGEventSource, CGEventSourceStateID},
    geometry::CGPoint,
};
use log::warn;

use super::{ButtonType, KeyState};
use crate::client::ScreenSize;

pub struct FakeDevice {
    source: CGEventSource,
    pressed_keys: HashSet<u16>,
    pending_shifts: HashSet<u16>,
    caps_lock: bool,
    caps_shifted_keys: HashSet<u16>,
    scroll_horizon_remainder: i32,
    scroll_vertical_remainder: i32,
    mouse_position: CGPoint,
    mouse_buttons: MouseButtons,
}

impl std::fmt::Debug for FakeDevice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FakeDevice")
            .field("pressed_keys", &self.pressed_keys)
            .field("pending_shifts", &self.pending_shifts)
            .field("caps_lock", &self.caps_lock)
            .field("caps_shifted_keys", &self.caps_shifted_keys)
            .field("scroll_horizon_remainder", &self.scroll_horizon_remainder)
            .field("scroll_vertical_remainder", &self.scroll_vertical_remainder)
            .field("mouse_position", &self.mouse_position)
            .field("mouse_buttons", &self.mouse_buttons)
            .finish()
    }
}

type Result<T> = std::io::Result<T>;

const CONTROL_KEYCODE: u16 = 59;
const COMMAND_KEYCODE: u16 = 55;
const RIGHT_COMMAND_KEYCODE: u16 = 54;
const LEFT_SHIFT_KEYCODE: u16 = 56;
const RIGHT_SHIFT_KEYCODE: u16 = 60;
const OPTION_KEYCODE: u16 = 58;
const RIGHT_OPTION_KEYCODE: u16 = 61;
const SPACE_KEYCODE: u16 = 49;
const NUMBER_4_KEYCODE: u16 = 21;
const SCROLL_UNIT: i32 = 120;
const SCROLL_STEPS_PER_NOTCH: i32 = 5;
const SCROLL_NOTCH_DELAY: Duration = Duration::from_millis(2);
const LETTER_KEYCODES: &[u16] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16, 17, 31, 32, 34, 35, 37, 38, 40, 45, 46,
];

enum MappedKey {
    Raw(u16),
    CapsLock,
    PrintScreen,
}

#[derive(Debug, Default)]
struct MouseButtons {
    left: bool,
    right: bool,
    center: bool,
}

#[derive(Clone, Copy)]
enum ScrollAxis {
    Horizontal,
    Vertical,
}

fn raw(keycode: u16) -> Option<MappedKey> {
    Some(MappedKey::Raw(keycode))
}

fn event_err() -> std::io::Error {
    std::io::Error::new(ErrorKind::Other, "failed to create CoreGraphics event")
}

fn state_convert(state: KeyState) -> bool {
    match state {
        KeyState::UP => false,
        KeyState::DOWN => true,
    }
}

fn is_shift_keycode(keycode: u16) -> bool {
    matches!(keycode, LEFT_SHIFT_KEYCODE | RIGHT_SHIFT_KEYCODE)
}

fn is_modifier_keycode(keycode: u16) -> bool {
    matches!(
        keycode,
        LEFT_SHIFT_KEYCODE
            | RIGHT_SHIFT_KEYCODE
            | CONTROL_KEYCODE
            | COMMAND_KEYCODE
            | RIGHT_COMMAND_KEYCODE
            | OPTION_KEYCODE
            | RIGHT_OPTION_KEYCODE
            | 57
    )
}

fn is_letter_keycode(keycode: u16) -> bool {
    LETTER_KEYCODES.contains(&keycode)
}

fn scancode_to_key(scancode: u16) -> Option<MappedKey> {
    match scancode {
        1 => raw(53), // Escape
        2 => raw(18), // 1
        3 => raw(19), // 2
        4 => raw(20), // 3
        5 => raw(NUMBER_4_KEYCODE),
        6 => raw(23),               // 5
        7 => raw(22),               // 6
        8 => raw(26),               // 7
        9 => raw(28),               // 8
        10 => raw(25),              // 9
        11 => raw(29),              // 0
        12 => raw(27),              // -
        13 => raw(24),              // =
        14 => raw(51),              // Backspace
        15 => raw(48),              // Tab
        16 => raw(12),              // Q
        17 => raw(13),              // W
        18 => raw(14),              // E
        19 => raw(15),              // R
        20 => raw(17),              // T
        21 => raw(16),              // Y
        22 => raw(32),              // U
        23 => raw(34),              // I
        24 => raw(31),              // O
        25 => raw(35),              // P
        26 => raw(33),              // [
        27 => raw(30),              // ]
        28 => raw(36),              // Return
        29 => raw(COMMAND_KEYCODE), // Left Control -> Command
        30 => raw(0),               // A
        31 => raw(1),               // S
        32 => raw(2),               // D
        33 => raw(3),               // F
        34 => raw(5),               // G
        35 => raw(4),               // H
        36 => raw(38),              // J
        37 => raw(40),              // K
        38 => raw(37),              // L
        39 => raw(41),              // ;
        40 => raw(39),              // '
        41 => raw(50),              // `
        42 => raw(LEFT_SHIFT_KEYCODE),
        43 => raw(42), // Backslash
        44 => raw(6),  // Z
        45 => raw(7),  // X
        46 => raw(8),  // C
        47 => raw(9),  // V
        48 => raw(11), // B
        49 => raw(45), // N
        50 => raw(46), // M
        51 => raw(43), // ,
        52 => raw(47), // .
        53 => raw(44), // /
        54 => raw(RIGHT_SHIFT_KEYCODE),
        55 => raw(67),             // Keypad *
        56 => raw(OPTION_KEYCODE), // Left Alt / Option
        57 => raw(SPACE_KEYCODE),  // Space
        58 => Some(MappedKey::CapsLock),
        59 => raw(122), // F1
        60 => raw(120), // F2
        61 => raw(99),  // F3
        62 => raw(118), // F4
        63 => raw(96),  // F5
        64 => raw(97),  // F6
        65 => raw(98),  // F7
        66 => raw(100), // F8
        67 => raw(101), // F9
        68 => raw(109), // F10
        69 => raw(71),  // Num Lock -> Keypad Clear
        70 => raw(113), // Scroll Lock -> F15
        71 => raw(89),  // Keypad 7
        72 => raw(91),  // Keypad 8
        73 => raw(92),  // Keypad 9
        74 => raw(78),  // Keypad -
        75 => raw(86),  // Keypad 4
        76 => raw(87),  // Keypad 5
        77 => raw(88),  // Keypad 6
        78 => raw(69),  // Keypad +
        79 => raw(83),  // Keypad 1
        80 => raw(84),  // Keypad 2
        81 => raw(85),  // Keypad 3
        82 => raw(82),  // Keypad 0
        83 => raw(65),  // Keypad .
        87 => raw(103), // F11
        88 => raw(111), // F12
        284 => raw(76), // Keypad Enter
        285 => raw(RIGHT_COMMAND_KEYCODE),
        309 => raw(75), // Keypad /
        311 => Some(MappedKey::PrintScreen),
        312 => raw(RIGHT_OPTION_KEYCODE),
        327 => raw(115), // Home
        328 => raw(126), // Up
        329 => raw(116), // Page Up
        331 => raw(123), // Left
        333 => raw(124), // Right
        335 => raw(119), // End
        336 => raw(125), // Down
        337 => raw(121), // Page Down
        338 => raw(114), // Insert / Help
        339 => raw(117), // Forward Delete
        347 => raw(COMMAND_KEYCODE),
        348 => raw(RIGHT_COMMAND_KEYCODE),
        349 => raw(110), // Menu key
        _ => None,
    }
}

fn button_convert(button: ButtonType) -> (CGMouseButton, CGEventType, CGEventType) {
    match button {
        ButtonType::Left => (
            CGMouseButton::Left,
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseUp,
        ),
        ButtonType::Middle => (
            CGMouseButton::Center,
            CGEventType::OtherMouseDown,
            CGEventType::OtherMouseUp,
        ),
        ButtonType::Right => (
            CGMouseButton::Right,
            CGEventType::RightMouseDown,
            CGEventType::RightMouseUp,
        ),
        ButtonType::Extra => (
            CGMouseButton::Center,
            CGEventType::OtherMouseDown,
            CGEventType::OtherMouseUp,
        ),
    }
}

impl FakeDevice {
    pub fn new(_screen_size: ScreenSize) -> Result<Self> {
        let source =
            CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| event_err())?;
        Ok(Self {
            source,
            pressed_keys: HashSet::new(),
            pending_shifts: HashSet::new(),
            caps_lock: false,
            caps_shifted_keys: HashSet::new(),
            scroll_horizon_remainder: 0,
            scroll_vertical_remainder: 0,
            mouse_position: CGPoint::new(0.0, 0.0),
            mouse_buttons: MouseButtons::default(),
        })
    }

    pub fn move_abs(&mut self, x: i32, y: i32) -> Result<()> {
        self.mouse_position = CGPoint::new(x as f64, y as f64);
        let event = CGEvent::new_mouse_event(
            self.source.clone(),
            self.mouse_move_type(),
            self.mouse_position,
            CGMouseButton::Left,
        )
        .map_err(|_| event_err())?;
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    pub fn button(&mut self, button: ButtonType, state: KeyState) -> Result<()> {
        let (button, down_type, up_type) = button_convert(button);
        let event_type = match state {
            KeyState::DOWN => {
                self.set_button_state(button, true);
                down_type
            }
            KeyState::UP => {
                self.set_button_state(button, false);
                up_type
            }
        };
        let event =
            CGEvent::new_mouse_event(self.source.clone(), event_type, self.mouse_position, button)
                .map_err(|_| event_err())?;
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    pub fn scroll(&mut self, horizon: i16, vertical: i16) -> Result<()> {
        self.scroll_horizon_remainder += horizon as i32;
        self.scroll_vertical_remainder += vertical as i32;

        let horizon_steps = self.scroll_horizon_remainder / SCROLL_UNIT;
        let vertical_steps = self.scroll_vertical_remainder / SCROLL_UNIT;
        self.scroll_horizon_remainder %= SCROLL_UNIT;
        self.scroll_vertical_remainder %= SCROLL_UNIT;

        self.scroll_notches(horizon_steps, ScrollAxis::Horizontal)?;
        self.scroll_notches(vertical_steps, ScrollAxis::Vertical)?;
        Ok(())
    }

    fn mouse_move_type(&self) -> CGEventType {
        if self.mouse_buttons.left {
            CGEventType::LeftMouseDragged
        } else if self.mouse_buttons.right {
            CGEventType::RightMouseDragged
        } else if self.mouse_buttons.center {
            CGEventType::OtherMouseDragged
        } else {
            CGEventType::MouseMoved
        }
    }

    fn set_button_state(&mut self, button: CGMouseButton, down: bool) {
        match button {
            CGMouseButton::Left => self.mouse_buttons.left = down,
            CGMouseButton::Right => self.mouse_buttons.right = down,
            CGMouseButton::Center => self.mouse_buttons.center = down,
        }
    }

    fn scroll_notches(&mut self, notches: i32, axis: ScrollAxis) -> Result<()> {
        let direction = notches.signum();
        for index in 0..notches.abs() {
            let (vertical, horizontal) = match axis {
                ScrollAxis::Vertical => (direction * SCROLL_STEPS_PER_NOTCH, 0),
                ScrollAxis::Horizontal => (0, direction * SCROLL_STEPS_PER_NOTCH),
            };
            let event = CGEvent::new_scroll_event(
                self.source.clone(),
                ScrollEventUnit::LINE,
                2,
                vertical,
                horizontal,
                0,
            )
            .map_err(|_| event_err())?;
            event.post(CGEventTapLocation::HID);
            if index + 1 < notches.abs() {
                thread::sleep(SCROLL_NOTCH_DELAY);
            }
        }
        Ok(())
    }

    fn key_flags(&self) -> CGEventFlags {
        let mut flags = CGEventFlags::empty();
        if self.pressed_keys.contains(&LEFT_SHIFT_KEYCODE)
            || self.pressed_keys.contains(&RIGHT_SHIFT_KEYCODE)
        {
            flags |= CGEventFlags::CGEventFlagShift;
        }
        if self.pressed_keys.contains(&COMMAND_KEYCODE)
            || self.pressed_keys.contains(&RIGHT_COMMAND_KEYCODE)
        {
            flags |= CGEventFlags::CGEventFlagCommand;
        }
        if self.pressed_keys.contains(&CONTROL_KEYCODE) {
            flags |= CGEventFlags::CGEventFlagControl;
        }
        if self.pressed_keys.contains(&OPTION_KEYCODE)
            || self.pressed_keys.contains(&RIGHT_OPTION_KEYCODE)
        {
            flags |= CGEventFlags::CGEventFlagAlternate;
        }
        flags
    }

    fn raw_key(&mut self, keycode: u16, down: bool, autorepeat: bool) -> Result<()> {
        let event = CGEvent::new_keyboard_event(self.source.clone(), keycode, down)
            .map_err(|_| event_err())?;
        event.set_flags(self.key_flags());
        if autorepeat {
            event.set_integer_value_field(
                core_graphics::event::EventField::KEYBOARD_EVENT_AUTOREPEAT,
                1,
            );
        }
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    fn tap_key(&mut self, keycode: u16, autorepeat: bool) -> Result<()> {
        self.raw_key(keycode, true, autorepeat)?;
        self.raw_key(keycode, false, false)
    }

    fn press_chord(&mut self, keycodes: &[u16]) -> Result<()> {
        for keycode in keycodes {
            self.raw_key(*keycode, true, false)?;
            self.pressed_keys.insert(*keycode);
        }

        for keycode in keycodes.iter().rev() {
            self.pressed_keys.remove(keycode);
            self.raw_key(*keycode, false, false)?;
        }

        Ok(())
    }

    fn flush_pending_shifts(&mut self) -> Result<()> {
        let pending: Vec<u16> = self.pending_shifts.drain().collect();
        for keycode in pending {
            self.pressed_keys.insert(keycode);
            self.raw_key(keycode, true, false)?;
        }
        Ok(())
    }

    fn switch_input_source(&mut self) -> Result<()> {
        self.press_chord(&[CONTROL_KEYCODE, SPACE_KEYCODE])
    }

    fn screenshot(&mut self) -> Result<()> {
        self.flush_pending_shifts()?;
        self.press_chord(&[
            COMMAND_KEYCODE,
            CONTROL_KEYCODE,
            LEFT_SHIFT_KEYCODE,
            NUMBER_4_KEYCODE,
        ])
    }

    fn has_active_shift(&self) -> bool {
        self.pressed_keys.contains(&LEFT_SHIFT_KEYCODE)
            || self.pressed_keys.contains(&RIGHT_SHIFT_KEYCODE)
            || !self.pending_shifts.is_empty()
    }

    fn maybe_press_caps_shift(&mut self, keycode: u16) -> Result<()> {
        if self.caps_lock && is_letter_keycode(keycode) && !self.has_active_shift() {
            self.raw_key(LEFT_SHIFT_KEYCODE, true, false)?;
            self.pressed_keys.insert(LEFT_SHIFT_KEYCODE);
            self.caps_shifted_keys.insert(keycode);
        }
        Ok(())
    }

    fn maybe_release_caps_shift(&mut self, keycode: u16) -> Result<()> {
        if self.caps_shifted_keys.remove(&keycode) {
            self.pressed_keys.remove(&LEFT_SHIFT_KEYCODE);
            self.raw_key(LEFT_SHIFT_KEYCODE, false, false)?;
        }
        Ok(())
    }

    pub fn keyboard(&mut self, keycode: u16, state: KeyState) -> Result<()> {
        let Some(key) = scancode_to_key(keycode) else {
            warn!("Unknown key: {}", keycode);
            return Ok(());
        };
        let MappedKey::Raw(keycode) = key else {
            if matches!(state, KeyState::DOWN) {
                match key {
                    MappedKey::CapsLock => self.caps_lock = !self.caps_lock,
                    MappedKey::PrintScreen => self.screenshot()?,
                    MappedKey::Raw(_) => unreachable!(),
                }
            }
            return Ok(());
        };

        if is_shift_keycode(keycode) {
            match state {
                KeyState::DOWN => {
                    self.pending_shifts.insert(keycode);
                    return Ok(());
                }
                KeyState::UP => {
                    if self.pending_shifts.remove(&keycode) {
                        return self.switch_input_source();
                    }
                }
            }
        }

        if matches!(state, KeyState::DOWN) {
            self.flush_pending_shifts()?;
        }

        let down = state_convert(state);
        if down {
            self.maybe_press_caps_shift(keycode)?;
            self.pressed_keys.insert(keycode);
        } else {
            self.pressed_keys.remove(&keycode);
        }

        self.raw_key(keycode, down, false)?;
        if !down {
            self.maybe_release_caps_shift(keycode)?;
        }
        Ok(())
    }

    pub fn repeat_keyboard(&mut self, keycode: u16) -> Result<()> {
        let Some(key) = scancode_to_key(keycode) else {
            warn!("Unknown repeat key: {}", keycode);
            return Ok(());
        };
        let MappedKey::Raw(keycode) = key else {
            return Ok(());
        };

        if is_shift_keycode(keycode) || is_modifier_keycode(keycode) {
            return Ok(());
        }

        self.flush_pending_shifts()?;
        self.maybe_press_caps_shift(keycode)?;
        self.tap_key(keycode, true)?;
        self.maybe_release_caps_shift(keycode)?;
        Ok(())
    }

    pub fn leave_screen(&mut self) -> Result<()> {
        self.pending_shifts.clear();
        self.caps_shifted_keys.clear();
        let pressed: Vec<u16> = self.pressed_keys.drain().collect();
        for key in pressed {
            self.raw_key(key, false, false)?;
        }
        Ok(())
    }
}
