#![feature(globs)]

extern crate xlib;

use config::get_config;
use workspaces::Workspaces;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::{ XMapRequest,
                          XConfigurationRequest,
                          XDestroy,
                          XUnmapNotify,
                          XEnterNotify,
                          XFocusOut,
                          XKeyPress};

mod config;
mod keycode;
mod commands;
mod xlib_window_system;
mod workspaces;
mod layout;


fn main() {
  let mut config = get_config();

  let ws = &mut XlibWindowSystem::new().unwrap();
  ws.grab_modifier(config.mod_key);

  let mut workspaces = Workspaces::new(ws, &mut config);
  workspaces.change_to(ws, &config, 0);

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
      XUnmapNotify(window) => {
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
      XKeyPress(_, mods, key) => {
        let mods = mods & !(config.mod_key | 0b10010);

        for binding in config.keybindings.iter() {
          if binding.mods == mods && binding.key == key {
            binding.cmd.run(ws, &mut workspaces, &config);
          }
        }
      },
      _ => {}
    }
  }
}
