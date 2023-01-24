#[macro_use]
extern crate log;

use anyhow::{Context, Result};
use config::Config;
use state::WmState;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::*;
use std::env;

mod commands;
mod config;
mod ewmh;
mod keycode;
mod layout;
mod stack;
mod state;
mod statusbar;
mod utils;
mod workspace;
mod xlib_window_system;

fn print_version() -> ! {
    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    ::std::process::exit(0);
}

fn print_help() -> ! {
    println!("usage: xr3wm [OPTION]
Xmonad and i3 inspired X11 tiling window manager.

  -c, --config=path config file path
  -h, --help        display this help and exit
  -v, --version     output version information and exit");
    ::std::process::exit(0);
}

fn handle_args() {
    let args = env::args().skip(1);

    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => print_help(),
            "--version" | "-v" => print_version(),
            _ => (),
        }
    }
}

fn run() -> Result<()> {
    handle_args();

    // initialize logging system
    env_logger::init();

    info!("loading config");

    let (config, ws_cfg_list) = Config::load()
        .context("failed to load config")?;

    info!("initializing Xlib");
    let xws = &mut XlibWindowSystem::new();
    xws.init();
    xws.grab_modifier(config.mod_key);

    let mut state = WmState::new(ws_cfg_list, xws)
        .context("failed to create initial wm state")?;

    state.rescreen(xws, &config);

    ewmh::set_current_desktop(xws, state.get_ws_index());
    ewmh::set_number_of_desktops(xws, state.ws_count());
    ewmh::set_desktop_names(xws, state.all_ws());
    ewmh::set_desktop_viewport(xws, state.all_ws());

    info!("entering event loop");
    run_event_loop(config, xws, state)
}

fn run_event_loop(config: Config, xws: &XlibWindowSystem, mut state: WmState) -> Result<()> {
    let mut bar_handle = config.statusbar
        .as_ref()
        .map(|bar| bar.start())
        .transpose()
        .context("failed to start statusbar")?;

    loop {
        match xws.get_event() {
            XMapRequest(window) => {
                trace!("XMapRequest: {:#x}", window);
                if !state.contains(window) {
                    let mut is_hooked = false;
                    if let Some(class) = xws.get_class_name(window) {
                        for hook in config.manage_hooks.iter() {
                            if hook.class_name == class {
                                hook.cmd.call(xws, &mut state, &config, window);
                                is_hooked = true;
                            }
                        }
                    }

                    if !is_hooked {
                        state.add_window(None, xws, &config, window);
                    }

                    state.focus_window(xws, &config, window, false);
                }
            }
            XMapNotify(window) => {
                trace!("XMapNotify: {:#x}", window);
                if xws.get_window_strut(window).is_some() {
                    state.add_strut(window);
                    state.redraw(xws, &config);
                }
            }
            XDestroy(window) => {
                trace!("XDestroy: {:#x}", window);
                if state.contains(window) {
                    state.remove_window(xws, &config, window);
                }
            }
            XUnmapNotify(window, send) => {
                trace!("XUnmapNotify: {:#x} {}", window, send);
                if send && state.contains(window) {
                    state.remove_window(xws, &config, window);
                } else if state.try_remove_strut(window) {
                    state.redraw(xws, &config);
                }
            }
            XPropertyNotify(window, atom, is_new_value) => {
                if atom == xws.get_atom("WM_HINTS") {
                    if let Some(ws) = state.get_parent_mut(window) {
                        ws.set_urgency(xws.is_urgent(window), window);
                    }
                } else if atom == xws.get_atom("_NET_WM_STRUT_PARTIAL") {
                    if is_new_value {
                        state.add_strut(window);
                    } else {
                        state.try_remove_strut(window);
                    }
                    state.redraw(xws, &config);
                } else if (window == xws.get_root_window() &&
                    (atom == xws.get_atom("_NET_CURRENT_DESKTOP") ||
                    atom == xws.get_atom("_NET_NUMBER_OF_DESKTOPS") ||
                    atom == xws.get_atom("_NET_DESKTOP_NAMES") ||
                    atom == xws.get_atom("_NET_ACTIVE_WINDOW"))) ||
                    atom == xws.get_atom("_NET_WM_STATE") ||
                    atom == xws.get_atom("_NET_WM_NAME")
                {
                    if let Some(ref mut handle) = bar_handle {
                        if let Err(e) = config.statusbar.as_ref().unwrap().update(handle, xws, &state) {
                            error!("{}", e.context("failed to update statusbar"));
                        }
                    }
                }
            }
            XClientMessage(window, msg_type, data) => {
                let data: Vec<u64> = data.as_longs().iter().map(|x| *x as u64).collect();
                trace!("ClientMessage: {:#x} {}, {:?}", window, xws.get_atom_name(msg_type), data);
                ewmh::process_client_message(&mut state, xws, &config, window, msg_type, &data);
            }
            XConfigureNotify(_) => {
                trace!("XConfigurationNotify");
                state.rescreen(xws, &config);
            }
            XConfigureRequest(window, changes, mask) => {
                trace!("XConfigureRequest: {:#x}", window);
                let unmanaged = state.is_unmanaged(window) || !state.contains(window);
                xws.configure_window(window, changes, mask, unmanaged);
            }
            XEnterNotify(window, is_root, x, y) => {
                trace!("XEnterNotify: {:#x} {} x: {} y: {}", window, is_root,x ,y);
                if is_root {
                    state.switch_to_ws_at(xws, &config, x, y, false)
                } else {
                    state.focus_window(xws, &config, window, false);
                }
            }
            XFocusIn(window) => {
                trace!("Focus event by: {:#x}", window);
                if let Some(idx) = state.find_window(window) {
                    let screens = state.get_screens();
                    if let Some(workspace) = state.get_ws(idx) {
                        workspace.redraw(xws, &config, screens);
                    }
                }
            }
            XButtonPress(window) => {
                state.focus_window(xws, &config, window, false);
            }
            XKeyPress(_, mods, key) => {
                trace!("XKeyPress: {}, {}", mods, key);
                let mods = mods & !(config.mod_key | 0b10010);

                for (binding, cmd) in config.keybindings.iter() {
                    if binding.mods == mods && binding.key == key {
                        cmd.call(xws, &mut state, &config, bar_handle.as_mut())
                            .map_err(|e| error!("{}", utils::concat_error_chain(&e)))
                            .ok();
                    }
                }
            }
            _ => {}
        }
    }

}

fn main() {
    // failure crate boilerplate
    if let Err(e) = run() {
        use std::io::Write;

        let mut stderr = std::io::stderr();

        if log_enabled!(log::Level::Error) {
            error!("{}", e);
        } else {
            writeln!(stderr, "ERROR: {e}").ok();
        }

        e.chain().skip(1)
            .for_each(|cause| error!("because: {}", cause));

        error!("backtrace: {}", e.backtrace());

        stderr.flush().ok();
        ::std::process::exit(1);
    }
}
