#![feature(phase)]
#[phase(plugin, link)]
extern crate log;
extern crate xlib;

use xlib_window_system::{XlibWindowSystem};

mod xlib_window_system;

fn main() {
  let mut ws = XlibWindowSystem::new().unwrap();
  ws.init();

  ws.event_loop();
}
