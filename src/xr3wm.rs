#![feature(globs)]

extern crate xlib;

use std::io::Command;
use config::get_config;
use workspaces::Workspaces;
use xlib_window_system::{ XlibWindowSystem,
                          XMapRequest,
                          XConfigurationRequest,
                          XDestroy,
                          XEnterNotify,
                          XFocusOut,
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
        let workspace = workspaces.get_current();
        workspace.add_window(ws, &config, window);
        workspace.focus_window(ws, &config, window);
      },
      XDestroy(window) => {
        workspaces.remove_window(ws, &config, window);
      },
      XConfigurationRequest(window, changes, mask) => {
        ws.configure_window(window, changes, mask);
      },
      XEnterNotify(window) => {
        workspaces.get_current().focus_window(ws, &config, window);
      },
      XFocusOut(_) => {
        workspaces.get_current().unfocus_window(ws, &config);
      },
      XKeyPress(_, keystroke) => {
        let key = keystroke.key;
        let mods = keystroke.mods ^ (config.mod_key | 0x10);
        let num_key : uint = from_str(key.as_slice()).unwrap_or(99);

        if num_key >= 1 && num_key <= config.workspaces.len() {
          workspaces.change_to(ws, num_key - 1);
        } else if key == config.terminal_shortcut.key && mods == config.terminal_shortcut.mods {
          let term = config.terminal.clone();
          spawn(proc() { Command::new(term).detached().spawn(); });
        } else if key == config.launcher_shortcut.key && mods == config.launcher_shortcut.mods {
          let launcher = config.launcher.clone();
          spawn(proc() { Command::new(launcher).detached().spawn(); });
        } else if key == config.kill_shortcut.key && mods == config.kill_shortcut.mods {
          ws.kill_window(workspaces.get_current().get_focused_window());
        }
      },
      _ => {}
    }
  }
}
