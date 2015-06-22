#![feature(path_ext)]
#[macro_use]

extern crate log;
extern crate xlib;
extern crate xinerama;

pub mod config;
pub mod keycode;
pub mod commands;
pub mod xlib_window_system;
pub mod workspaces;
pub mod layout;