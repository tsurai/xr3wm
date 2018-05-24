#[macro_use]

extern crate log;
extern crate env_logger;
extern crate dylib;
extern crate xlib;
extern crate xinerama;

use config::Config;
use workspaces::Workspaces;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::{XMapRequest, XConfigurationNotify, XConfigurationRequest,
                                    XDestroy, XUnmapNotify, XPropertyNotify, XEnterNotify,
                                    XFocusOut, XKeyPress, XButtonPress};

mod config;
mod keycode;
mod commands;
mod xlib_window_system;
mod workspaces;
mod layout;

fn main() {
    env_logger::init();

    let mut config = Config::default();
    config = Config::load();

    let ws = &XlibWindowSystem::new();
    ws.grab_modifier(config.mod_key);

    let mut workspaces = Workspaces::new(&config, ws.get_screen_infos().len());

    if let Some(ref mut statusbar) = (&mut config).statusbar {
        statusbar.start();
    }

    loop {
        match ws.get_event() {
            XMapRequest(window) => {
                debug!("XMapRequest: {}", window);
                if !workspaces.contains(window) {
                    let class = ws.get_class_name(window);
                    let mut is_hooked = false;

                    for hook in config.manage_hooks.iter() {
                        if hook.class_name == class {
                            is_hooked = true;
                            hook.cmd.call(ws, &mut workspaces, &config, window);
                        }
                    }

                    if !is_hooked {
                        workspaces.current_mut().add_window(ws, &config, window);
                        workspaces.current_mut().focus_window(ws, &config, window);
                    }
                }
            }
            XDestroy(window) => {
                if workspaces.contains(window) {
                    debug!("XDestroy: {}", window);
                    workspaces.remove_window(ws, &config, window);
                }
            }
            XUnmapNotify(window, send) => {
                if send && workspaces.contains(window) {
                    debug!("XUnmapNotify: {}", window);
                    workspaces.remove_window(ws, &config, window);
                }
            }
            XPropertyNotify(window, atom, _) => {
                if atom == ws.get_atom("WM_HINTS") {
                    if let Some(workspace) = workspaces.find_window(window) {
                        workspace.set_urgency(ws.is_urgent(window), ws, &config, window);
                    }
                }
            }
            XConfigurationNotify(_) => {
                workspaces.rescreen(ws, &config);
            }
            XConfigurationRequest(window, changes, mask) => {
                let unmanaged = workspaces.is_unmanaged(window) || !workspaces.contains(window);
                ws.configure_window(window, changes, mask, unmanaged);
            }
            XEnterNotify(window) => {
                debug!("XEnterNotify: {}", window);
                workspaces.focus_window(ws, &config, window);
            }
            XFocusOut(_) => {
                debug!("XFocusOut");
                workspaces.current_mut().unfocus_window(ws, &config);
            }
            XButtonPress(window) => {
                debug!("XButtonPress: {}", window);
                workspaces.focus_window(ws, &config, window);
            }
            XKeyPress(_, mods, key) => {
                debug!("XKeyPress: {}, {}", mods, key);
                let mods = mods & !(config.mod_key | 0b10010);

                for binding in config.keybindings.iter() {
                    if binding.mods == mods && binding.key == key {
                        binding.cmd.call(ws, &mut workspaces, &config);
                    }
                }
            }
            _ => {}
        }

        if let Some(ref mut statusbar) = (&mut config).statusbar {
            statusbar.update(ws, &workspaces);
        }
    }
}
