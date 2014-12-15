#![feature(globs)]

extern crate xlib;
extern crate xinerama;

use config::get_config;
use workspaces::Workspaces;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::{ XMapRequest,
                          XConfigurationNotify,
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
        if !workspaces.contains(window) {
          let class = ws.get_class_name(window);

          workspaces.current().add_window(ws, config, window);
          workspaces.current().focus_window(ws, config, window);

          for hook in config.manage_hooks.iter() {
            if hook.class_name == class {
              hook.cmd.call(ws, &mut workspaces, config, window);
            }
          }
        }
      },
      XDestroy(window) => {
        workspaces.remove_window(ws, config, window);
      },
      XUnmapNotify(window) => {
        workspaces.remove_window(ws, config, window);
      },
      XConfigurationNotify(_) => {
        workspaces.reconfigure(ws, config);
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
            binding.cmd.call(ws, &mut workspaces, config);
          }
        }
      },
      _ => {}
    }
  }
}
