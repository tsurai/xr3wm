#![feature(globs)]

extern crate xlib;

use keycode::MOD_SHIFT;
use std::io::process::Command;
use config::get_config;
use workspaces::{Workspaces, MoveOp};
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::{ XMapRequest,
                          XConfigurationRequest,
                          XDestroy,
                          XUnmapNotify,
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
  workspaces.change_to(ws, &config, 0);

  loop {
    match ws.get_event() {
      XMapRequest(window) => {
        let workspace = workspaces.get_current();
        workspace.add_window(ws, &config, window);
        workspace.focus_window(ws, &config, window);
      },
      XUnmapNotify(window) => {
        workspaces.remove_window(ws, &config, window);
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
        let mods = keystroke.mods & !(config.mod_key | 0b10010);
        let num_key : uint = from_str(key.as_slice()).unwrap_or(99);

        if num_key >= 1 && num_key <= config.workspaces.len() {
          if mods == MOD_SHIFT {
            workspaces.move_window_to(ws, &config, num_key - 1);
          } else {
            workspaces.change_to(ws, &config, num_key - 1);
          }
        } else if key == String::from_str("k") && mods == 0 {
          workspaces.get_current().move_focus_up(ws, &config);
        } else if key == String::from_str("j") && mods == 0 {
          workspaces.get_current().move_focus_down(ws, &config);
        } else if key == String::from_str("k") && mods == MOD_SHIFT {
          workspaces.get_current().move_window(ws, &config, MoveOp::Up);
        } else if key == String::from_str("j") && mods == MOD_SHIFT {
          workspaces.get_current().move_window(ws, &config, MoveOp::Down);
        } else if key == String::from_str("Return") && mods == MOD_SHIFT {
          workspaces.get_current().move_window(ws, &config, MoveOp::Swap);
        } else if key == config.terminal_shortcut.key && mods == config.terminal_shortcut.mods {
          let term = config.terminal.clone();
          spawn(proc() {
            match Command::new(term).detached().spawn() {
              Ok(_) => (),
              _ => panic!("failed to start terminal")
            }
          });
        } else if key == config.launcher_shortcut.key && mods == config.launcher_shortcut.mods {
          let launcher = config.launcher.clone();
          spawn(proc() {
            match Command::new(launcher).detached().spawn() {
              Ok(_) => (),
              _ => panic!("failed to start launcher")
            }
          });
        } else if key == config.kill_shortcut.key && mods == config.kill_shortcut.mods {
          ws.kill_window(workspaces.get_current().get_focused_window());
        } else if key == String::from_str("e") && mods == MOD_SHIFT {
          break;
        }
      },
      _ => {}
    }
  }
}
