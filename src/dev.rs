use std::{collections::HashSet, io::ErrorKind};
use log::{warn};
use evdev_rs::{
    AbsInfo, DeviceWrapper, EnableCodeData, InputEvent, TimeVal, UInputDevice, UninitDevice, enums::*
};

use crate::{client::ScreenSize, helper};

pub const ZERO_TIMEVAL: TimeVal = TimeVal::new(0, 0);

#[derive(Debug)]
pub struct FakeDevice {
    dev: UInputDevice,
    pressed_keys: HashSet<EV_KEY>,
}

pub enum KeyState {
    UP = 0,
    DOWN = 1,
}

type Result<T> = std::io::Result<T>;

impl FakeDevice {
    pub fn new(screen_size: ScreenSize) -> std::io::Result<Self> {
        // Create virtual device
        let u = UninitDevice::new().expect("evdev error. This should not happen.");
        // Setup device
        // per: https://01.org/linuxgraphics/gfx-docs/drm/input/uinput.html#mouse-movements
        u.set_name("Virtual Mouse");
        u.set_bustype(BusType::BUS_USB as u16);
        u.set_vendor_id(0xabcd);
        u.set_product_id(0xefef);

        // Note mouse keys have to be enabled for this to be detected
        // as a usable device, see: https://stackoverflow.com/a/64559658/6074942
        
        for i in 0u32..helper::KEYCODE_LIMIT {
            if let Some(code) = evdev_rs::enums::int_to_ev_key(i) {
                u.enable(EventCode::EV_KEY(code))?;
            }
        }

        //三个键
        u.enable(EventCode::EV_KEY(EV_KEY::BTN_LEFT))?;
        u.enable(EventCode::EV_KEY(EV_KEY::BTN_RIGHT))?;
        u.enable(EventCode::EV_KEY(EV_KEY::BTN_MIDDLE))?;

        //滚轮
        u.enable(EventCode::EV_REL(EV_REL::REL_WHEEL_HI_RES))?;  //垂直滚轮
        u.enable(EventCode::EV_REL(EV_REL::REL_HWHEEL_HI_RES))?; //水平滚轮 ???

        //鼠标移动, 相对和绝对坐标
        u.enable(EventCode::EV_REL(EV_REL::REL_X))?;
        u.enable(EventCode::EV_REL(EV_REL::REL_Y))?;

        // 绝对值坐标需要absinfo
        // pub struct AbsInfo {
        //     pub value: i32,
        //     pub minimum: i32,
        //     pub maximum: i32,
        //     pub fuzz: i32,
        //     pub flat: i32,
        //     pub resolution: i32,
        // }
        let abs_x_info = AbsInfo {
            value: 0, minimum: 0, maximum: screen_size.x as i32, fuzz: 0, flat: 0, resolution: 1
        };
        let abs_y_info = AbsInfo {
            value: 0, minimum: 0, maximum: screen_size.y as i32, fuzz: 0, flat: 0, resolution: 1
        };
        u.enable_event_code(&EventCode::EV_ABS( EV_ABS::ABS_X ),
        Some(EnableCodeData::AbsInfo(abs_x_info)) )?;
        u.enable_event_code(&EventCode::EV_ABS( EV_ABS::ABS_Y ),
        Some(EnableCodeData::AbsInfo(abs_y_info)) )?;
                
        // u.enable(EventCode::EV_ABS(EV_ABS::ABS_X))?;
        // u.enable(EventCode::EV_ABS(EV_ABS::ABS_Y))?;

        u.enable(EventCode::EV_SYN(EV_SYN::SYN_REPORT))?;

        // Attempt to create UInputDevice from UninitDevice
        let dev = UInputDevice::create_from_device(&u)?;
        Ok( Self { dev, pressed_keys: HashSet::with_capacity(128) } )
    }

    #[allow(unused)]
    pub fn move_rel(&self, x: i32, y: i32, time: evdev_rs::TimeVal) -> std::io::Result<()> {
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_REL(EV_REL::REL_X),
            value: x,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_REL(EV_REL::REL_Y),
            value: y,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;
        Ok(())
    }

    pub fn move_abs(&self, x: i32, y: i32, time: evdev_rs::TimeVal) -> std::io::Result<()> {
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_ABS(EV_ABS::ABS_X),
            value: x,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_ABS(EV_ABS::ABS_Y),
            value: y,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;
        Ok(())
    }

    fn check_button(button: EV_KEY) -> bool {
        matches!(button, EV_KEY::BTN_LEFT | EV_KEY::BTN_RIGHT | EV_KEY::BTN_MIDDLE)
    }

    pub fn button(&self, button: EV_KEY, state: KeyState, time: TimeVal) -> std::io::Result<()> {
        if !Self::check_button(button) {
            return Err(std::io::Error::new(ErrorKind::InvalidData, "It is not a mouse button"));
        }
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_KEY(button),
            value: state as i32,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;
        Ok(())
    }

    pub fn scroll(&self, horizon: i16, vertical: i16, time: TimeVal) -> Result<()> {
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_REL(EV_REL::REL_HWHEEL_HI_RES),
            value: horizon as i32,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_REL(EV_REL::REL_WHEEL_HI_RES),
            value: vertical as i32,
        })?;
        self.dev.write_event(&InputEvent {
            time,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;
        Ok(())
    }

    // Keyboard

    pub fn keyboard(&mut self, keycode: u16, state: KeyState, time: TimeVal) -> Result<()> {
        // 存在bug: 离开屏幕后按键不会弹起
        // 决定维护一个按键表, COUT信号后清除所有按下的按键
        if let Some(code) = helper::scancode_to_keycode(keycode) {
            match state {
                KeyState::UP => self.pressed_keys.remove(&code),
                KeyState::DOWN => self.pressed_keys.insert(code),
            };
            self.dev.write_event(&InputEvent {
                time,
                event_code: EventCode::EV_KEY(code),
                value: state as i32,
            })?;
            self.dev.write_event(&InputEvent {
                time,
                event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                value: 0,
            })?;
            // trace!("Press: {}", keycode);
        } else {
                warn!("Unknown key: {}", keycode);
        }
        Ok(())
    }

    /// 离开屏幕时调用, 清空所有按下的按键
    pub fn leave_screen(&mut self) -> Result<()> {
        for key in self.pressed_keys.iter() {
            self.dev.write_event(&InputEvent {
                time: ZERO_TIMEVAL,
                event_code: EventCode::EV_KEY(*key),
                value: 0,
            })?;
        }
        self.pressed_keys.clear();
        self.dev.write_event(&InputEvent {
            time: ZERO_TIMEVAL,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;
        Ok(())
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uinput_test() {
        let _ = FakeDevice::new(ScreenSize { x: (1920), y: (1080) }).unwrap();
    }
}
