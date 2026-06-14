use std::{collections::HashSet, io::ErrorKind};
use enigo::{
    Direction, Enigo, Keyboard, Mouse, Settings
};

use crate::{client::ScreenSize};
use super::{ ButtonType, KeyState };

#[derive(Debug)]
pub struct FakeDevice {
    dev: Enigo,
    pressed_keys: HashSet<u16>,
    screen_size: ScreenSize,
}


type Result<T> = std::io::Result<T>;

fn to_io_err<E>(error: E) -> std::io::Error
where E: Into<Box<dyn std::error::Error + Send + Sync>>  {
    std::io::Error::new(ErrorKind::NotFound, error)   
}

fn convert<T, E>(thing: std::result::Result<T, E>) -> Result<T>
where E: Into<Box<dyn std::error::Error + Send + Sync>>  {
    match thing {
        Ok(thing) => Ok(thing),
        Err(err) => Err(to_io_err(err)),
    }
}

fn button_convert(button: ButtonType) -> enigo::Button {
    match button {
        ButtonType::Left => enigo::Button::Left,
        ButtonType::Middle => enigo::Button::Middle,
        ButtonType::Right => enigo::Button::Right,
        ButtonType::Extra => enigo::Button::Back,
    }
}

fn state_convert(state: KeyState) -> enigo::Direction {
    match state {
        KeyState::UP => enigo::Direction::Release,
        KeyState::DOWN => enigo::Direction::Press,
    }
}

impl FakeDevice {
    pub fn new(screen_size: ScreenSize) -> Result<Self> {
        let enigo = convert(Enigo::new(&Settings::default()))?;
        Ok(Self {
            dev: enigo,
            pressed_keys: HashSet::new(),
            screen_size,
        })
    }

    pub fn move_abs(&mut self, x: i32, y: i32) -> Result<()> {
        convert(self.dev.move_mouse(x, y, enigo::Coordinate::Abs))?;
        Ok(())
    }

    pub fn button(&mut self, button: ButtonType, state: KeyState) -> Result<()> {
        convert( self.dev.button(button_convert(button), state_convert(state)))
    }

    pub fn scroll(&mut self, horizon: i16, vertical: i16) -> Result<()> {
        convert( self.dev.scroll(horizon as i32, enigo::Axis::Horizontal))?;
        convert(self.dev.scroll(vertical as i32, enigo::Axis::Vertical))
    }

    // Keyboard

    pub fn keyboard(&mut self, keycode: u16, state: KeyState) -> Result<()> {
        // 存在bug: 离开屏幕后按键不会弹起
        // 决定维护一个按键表, COUT信号后清除所有按下的按键
        match state {
            KeyState::DOWN => self.pressed_keys.insert(keycode),
            KeyState::UP => self.pressed_keys.remove(&keycode),
        };
        convert(self.dev.raw(keycode, state_convert(state)))?;
        Ok(())
    }

    /// 离开屏幕时调用, 清空所有按下的按键
    pub fn leave_screen(&mut self) -> Result<()> {
        for key in self.pressed_keys.iter() {
            convert(self.dev.raw(*key, Direction::Release))?;
        }
        self.pressed_keys.clear();
        Ok(())
    }

}

