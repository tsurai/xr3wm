#![allow(dead_code)]

use std::cmp::Eq;

pub const MOD_SHIFT: u8 = 1;
pub const MOD_LOCK: u8 = 1 << 1;
pub const MOD_CONTROL: u8 = 1 << 2;
pub const MOD_1: u8 = 1 << 3;
pub const MOD_2: u8 = 1 << 4;
pub const MOD_3: u8 = 1 << 5;
pub const MOD_4: u8 = 1 << 6;
pub const MOD_5: u8 = 1 << 7;

#[derive(PartialEq, Hash)]
pub struct Keybinding {
    pub mods: u8,
    pub key: String,
}

impl Eq for Keybinding {}
