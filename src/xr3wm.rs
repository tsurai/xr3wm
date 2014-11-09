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

mod keycode;
mod xlib_window_system;
mod workspaces;
mod layout;
mod config;

fn main() {
  let config = get_config();

  let ws = &mut XlibWindowSystem::new().unwrap();
  ws.grab_modifier(config.mod_key);

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
      XKeyPress(window, keystroke) => {
        let num_key : uint = from_str(keystroke.key.as_slice()).unwrap_or(99);

        if num_key >= 1 && num_key <= config.workspaces.len() {
          workspaces.change_to(ws, num_key - 1);
        } else if keystroke.key == config.terminal_shortcut.key {
          let term = config.terminal.clone();
          spawn(proc() { Command::new(term).detached().spawn(); });
        } else if keystroke.key == config.launcher_shortcut.key {
          let launcher = config.launcher.clone();
          spawn(proc() { Command::new(launcher).detached().spawn(); });
        }
      },
      _ => {}
    }
  }
}
