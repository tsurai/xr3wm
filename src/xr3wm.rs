#![feature(phase)]
#[phase(plugin, link)]
extern crate log;

use xlib_window_system::XlibWindowSystem;

mod xlib_window_system;

fn main() {
  let mut ws = XlibWindowSystem::new().unwrap();

  info!("starting xr3wm");
}
