#![feature(phase)]
#[phase(plugin, link)]
extern crate log;
extern crate xlib;

use xlib_window_system::{XlibWindowSystem, XMapRequest};

mod xlib_window_system;

fn main() {
  let ws = XlibWindowSystem::new().unwrap();
  info!("starting xr3wm");

  ws.event_loop();
}
