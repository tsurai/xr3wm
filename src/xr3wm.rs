#![feature(globs)]

extern crate xlib;
extern crate xinerama;

use config::get_config;
use workspaces::Workspaces;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::{ XMapRequest,
                          XConfigurationRequest,
                          XDestroy,
                          XUnmapNotify,
                          XEnterNotify,
                          XFocusOut,
                          XKeyPress,
                          XButtonPress};

mod config;
mod keycode;
mod commands;
mod xlib_window_system;
mod workspaces;
mod layout;


fn main() {
  let mut config = &mut get_config();

  let ws = &mut XlibWindowSystem::new().unwrap();
  ws.grab_modifier(config.mod_key);

  let mut workspaces = Workspaces::new(config, ws.get_screen_infos().len());

  loop {
    match ws.get_event() {
      XMapRequest(window) => {
        let workspace = workspaces.current();

        workspace.add_window(ws, config, window);
        workspace.focus_window(ws, config, window);
      },
      XDestroy(window) => {
        workspaces.remove_window(ws, config, window);
      },
      XUnmapNotify(window) => {
        workspaces.remove_window(ws, config, window);
      },
      XConfigurationRequest(window, changes, mask) => {
        ws.configure_window(window, changes, mask);
      },
      XEnterNotify(window) => {
        workspaces.current().focus_window(ws, config, window);
      },
      XFocusOut(_) => {
        workspaces.current().unfocus_window(ws, config);
      },
      XButtonPress(window) => {
        workspaces.current().focus_window(ws, config, window);
      },
      XKeyPress(_, mods, key) => {
        let mods = mods & !(config.mod_key | 0b10010);

        for binding in config.keybindings.iter() {
          if binding.mods == mods && binding.key == key {
            binding.cmd.run(ws, &mut workspaces, config);
          }
        }
      },
      _ => {}
    }
  }
}
