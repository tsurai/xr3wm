#[macro_use]
extern crate log;
extern crate fern;
extern crate failure;
extern crate clap;
extern crate libloading;
extern crate xlib;
extern crate xinerama;

use clap::{Arg, App, ArgMatches};
use clap::AppSettings::*;
use failure::{ResultExt, Error, Fail};
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
mod utils;

fn process_cli<'a>() -> ArgMatches<'a> {
    App::new("xr3wm")
        .version("0.0.1")
        .author("Cristian Kubis <cristian.kubis@tsunix.de>")
        .about("i3wm inspired tiling window manager")
        .setting(DeriveDisplayOrder)
        .arg(Arg::with_name("verbose")
             .short("v")
             .long("verbose")
             .multiple(true)
             .help("increrases the logging verbosity each use for up to 2 times"))
        .get_matches()
}

// initialization of the logging system
fn init_logger(verbosity: u64, logfile: &str) -> Result<(), Error> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
        // set the verbosity of the logging
        .level(match verbosity {
            1 => log::LevelFilter::Debug,
            x if x > 1 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info
        })
        // output everything but errors to stdout
        .chain(
            fern::Dispatch::new()
                .filter(move |metadata| metadata.level() != log::LevelFilter::Error)
                .chain(::std::io::stdout()))
        // ...and to a logfile with additional timestamps
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Error)
                .chain(
                    fern::Dispatch::new()
                        .chain(::std::io::stderr())
                .chain(fern::log_file(logfile)
                       .context("failed to open log file")?)))
        .apply()
        .map_err(|e| e.into())
}

fn run() -> Result<(), Error> {
    let matches = process_cli();

    let verbosity = matches.occurrences_of("verbose");

    // initialize logging system
    if let Err(e) = init_logger(verbosity, concat!(env!("HOME"), "/.xr3wm/xr3wm.log")) {
        println!("[ERROR] failed to initialize logging system: {}", e);
        ::std::process::exit(1);
    }

    info!("loading config");
    let mut config = Config::load()
        .map_err(|e| {
            let error = utils::concat_error_chain(&e);
            utils::xmessage(&format!("failed to load config:\n{}", error))
                .map_err(|e| warn!("failed to run xmessage: {}", e))
                .ok();
            e
        })
        .context("failed to load config")?;

    let ws = &XlibWindowSystem::new();
    ws.grab_modifier(config.mod_key);

    let workspaces = Workspaces::new(&config, ws.get_screen_infos().len());

    if let Some(ref mut statusbar) = config.statusbar {
        statusbar.start()
            .context("failed to start statusbar")?;
    }

    info!("entering event loop");
    run_event_loop(config, &ws, workspaces)
}

fn run_event_loop(mut config: Config, ws: &XlibWindowSystem, mut workspaces: Workspaces) -> Result<(), Error> {
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
                        binding.cmd.call(ws, &mut workspaces, &config)
                            .map_err(|e| error!("failed to execute binding call: {}", e))
                            .ok();
                    }
                }
            }
            _ => {}
        }

        if let Some(ref mut statusbar) = config.statusbar {
            if let Err(e) = statusbar.update(ws, &workspaces) {
                error!("{}", e.context("failed to update statusbar"));
            }
        }
    }

}

fn main() {
    // failure crate boilerplate
    if let Err(e) = run() {
        use std::io::Write;
        let mut stderr = std::io::stderr();
        let got_logger = log_enabled!(log::Level::Error);

        let mut fail: &dyn Fail = e.as_fail();
        if got_logger {
            error!("{}", fail);
        } else {
            writeln!(&mut stderr, "{}", fail).ok();
        }

        while let Some(cause) = fail.cause() {
            if got_logger {
                error!("caused by: {}", cause);
            } else {
                writeln!(&mut stderr, "caused by: {}", cause).ok();
            }

            if let Some(bt) = cause.backtrace() {
                error!("backtrace: {}", bt)
            }
            fail = cause;
        }

        stderr.flush().ok();
        ::std::process::exit(1);
    }
}
