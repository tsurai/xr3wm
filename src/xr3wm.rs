extern crate xlib;
extern crate serialize;

use std::io::Command;
use config::get_config;
use workspaces::Workspaces;
use xlib_window_system::{ XlibWindowSystem,
                          XMapRequest,
                          XConfigurationRequest,
                          XDestroyNotify,
                          XEnterNotify,
                          XLeaveNotify,
                          XKeyPress};

mod xlib_window_system;
mod workspaces;
mod layout;
mod keysym;
mod config;

fn main() {
  let ws = &mut XlibWindowSystem::new().unwrap();

  let config = get_config();

  let mut workspaces = Workspaces::new(ws, &config);
  workspaces.change_to(ws, 0);

  loop {
    match ws.get_event() {
      XMapRequest(window) => {
        workspaces.get_current().add_window(ws, &config, window);
      },
      XDestroyNotify(window) => {
        workspaces.remove_window(ws, &config, window);
      },
      XConfigurationRequest(window, changes, mask) => {
        ws.configure_window(window, changes, mask);
      },
      XEnterNotify(window) => {
        ws.set_window_border_color(window, config.border_color);
      },
      XLeaveNotify(window) => {
        ws.set_window_border_color(window, config.border_focus_color);
      },
      XKeyPress(window, state, keycode) => {
        if state == 80 {
          if keycode > 9 && keycode < 19 {
            workspaces.change_to(ws, keycode as uint - 10);
          } else if keycode == 36 {
            let term = config.terminal.clone();
            spawn(proc() { Command::new(term).spawn(); });
          }
        }
      },
      _ => {}
    }
  }
}
