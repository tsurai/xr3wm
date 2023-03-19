use crate::xlib_window_system::XlibWindowSystem;
use crate::workspace::Workspace;
use crate::state::WmState;
use crate::config::Config;
use x11::xlib::*;
use std::ffi::CString;

pub const NET_WM_STATE_REMOVE: u64 = 0;
pub const NET_WM_STATE_ADD: u64 = 1;
pub const NET_WM_STATE_TOGGLE: u64 = 2;

pub fn init_ewmh(xws: &mut XlibWindowSystem) {
    debug!("initializing ewmh");
    let root = xws.get_root_window();

    let atoms = &["_NET_SUPPORTED", "_NET_ACTIVE_WINDOW", "_NET_CLIENT_LIST",
        "_NET_CURRENT_DESKTOP", "_NET_DESKTOP_NAMES", "_NET_DESKTOP_VIEWPORT",
        "_NET_NUMBER_OF_DESKTOPS", "_NET_SUPPORTING_WM_CHECK", "_NET_WM_NAME",
        "_NET_WM_STATE", "_NET_WM_STATE_FULLSCREEN", "_NET_WM_STRUT_PARTIAL",
        "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_DOCK"];

    xws.cache_atoms(atoms);

    let atoms: Vec<Atom> = atoms.iter().map(|x| xws.get_atom(x)).collect();
    xws.change_property(root, "_NET_SUPPORTED", XA_ATOM, PropModeReplace, &atoms);

    let window = xws.create_hidden_window();
    xws.change_property(root, "_NET_SUPPORTING_WM_CHECK", XA_WINDOW, PropModeReplace, &[window]);
    xws.change_property(window, "_NET_SUPPORTING_WM_CHECK", XA_WINDOW, PropModeReplace, &[window]);

    let wm_name = CString::new("xr3wm").unwrap();
    xws.change_property(window, "_NET_WM_NAME", "UTF8_STRING", PropModeReplace, wm_name.as_bytes_with_nul());
}

#[allow(dead_code)]
pub fn process_client_message(state: &mut WmState, xws: &XlibWindowSystem, config: &Config, window: Window, msg_type: Atom, msg_data: &[u64]) {
    match xws.get_atom_name(msg_type).as_str() {
        "_NET_ACTIVE_WINDOW" => {
            state.focus_window(xws, config, window, true);
        },
        "_NET_CURRENT_DESKTOP" => {
            state.switch_to_ws(xws, config, msg_data[0] as usize, true);
        },
        "_NET_WM_STATE" => {
            let mode = msg_data[0];
            let wm_states: Vec<u64> = msg_data[1..3].iter()
                .filter(|&x| *x != 0)
                .cloned()
                .collect();

            let mut redraw = false;
            let ret = set_wm_state(xws, window, &wm_states, mode);

            if let Some((add_states, rem_states)) = ret {
                if add_states.contains(&xws.get_atom("_NET_WM_STATE_DEMANDS_ATTENTION")) {
                    state.set_urgency(true, window);
                    redraw = true;
                }

                if rem_states.contains(&xws.get_atom("_NET_WM_STATE_DEMANDS_ATTENTION")) {
                    state.set_urgency(false, window);
                    redraw = true;
                }

                if add_states.iter().chain(rem_states.iter()).any(|&x| x == xws.get_atom("_NET_WM_STATE_FULLSCREEN")) {
                    redraw = true;
                }
            }

            if redraw {
                state.redraw(xws, config);
            }
        },
        _ => {}
    }
}

pub fn set_active_window(xws: &XlibWindowSystem, window: Window) {
    trace!("set active window: {:#x}", window);
    let root = xws.get_root_window();
    xws.change_property(root, "_NET_ACTIVE_WINDOW", XA_WINDOW, PropModeReplace, &[window]);
}

pub fn get_active_window(xws: &XlibWindowSystem) -> Option<Window> {
    let root = xws.get_root_window();
    xws.get_property(root, "_NET_ACTIVE_WINDOW")
        .map(|x| x[0])
}

#[allow(dead_code)]
pub fn set_number_of_desktops(xws: &XlibWindowSystem, num_desktops: usize) {
    let root = xws.get_root_window();
    xws.change_property(root, "_NET_NUMBER_OF_DESKTOPS", XA_CARDINAL, PropModeReplace, &[num_desktops as u64]);
}

pub fn set_current_desktop(xws: &XlibWindowSystem, index: usize) {
    let root = xws.get_root_window();
    xws.change_property(root, "_NET_CURRENT_DESKTOP", XA_CARDINAL, PropModeReplace, &[index as u64]);
}

#[allow(dead_code)]
pub fn set_desktop_names(xws: &XlibWindowSystem, workspaces: &[Workspace]) {
    let root = xws.get_root_window();

    let names: Vec<Vec<u8>>= workspaces
        .iter()
        .map(|x| x.get_tag().to_owned())
        .filter_map(|x| CString::new(x).ok())
        .map(|x| x.into_bytes_with_nul())
        .collect();

    xws.change_property(root, "_NET_DESKTOP_NAMES", "UTF8_STRING", PropModeReplace, &names.as_slice().concat());
}

pub fn set_desktop_viewport(xws: &XlibWindowSystem, workspaces: &[Workspace]) {
    let root = xws.get_root_window();
    let screens = xws.get_screen_infos();

    let viewports: Vec<u64> = workspaces.iter()
        .map(|ws| {
            screens
                .get(ws.get_screen())
                .map(|s| vec![s.x as u64, s.y as u64])
                .unwrap_or_else(|| vec![0u64, 0])
        })
        .collect::<Vec<Vec<u64>>>()
        .as_slice()
        .concat();

    xws.change_property(root, "_NET_DESKTOP_VIEWPORT", XA_CARDINAL, PropModeReplace, &viewports);
}

pub fn set_client_list(xws: &XlibWindowSystem, workspaces: &[Workspace]) {
    let root = xws.get_root_window();

    let clients: Vec<Window> = workspaces.iter()
        .map(|ws| ws.all())
        .collect::<Vec<Vec<Window>>>()
        .as_slice()
        .concat();

    xws.change_property(root, "_NET_CLIENT_LIST", XA_WINDOW, PropModeReplace, &clients);
}

pub fn set_wm_state(xws: &XlibWindowSystem, window: Window, atoms: &[Atom], mode: u64) -> Option<(Vec<u64>, Vec<u64>)> {
    let active_atoms = xws.get_property(window, "_NET_WM_STATE")
        .unwrap_or_default();

    match mode {
        NET_WM_STATE_REMOVE => {
            xws.change_property(window, "_NET_WM_STATE", XA_ATOM, PropModeReplace, &active_atoms.into_iter().filter(|x| !atoms.iter().any(|y| y == x)).collect::<Vec<u64>>());
            Some((vec![], atoms.to_vec()))
        },
        NET_WM_STATE_ADD | NET_WM_STATE_TOGGLE => {
            let (add_atoms, rem_atoms) = if mode == NET_WM_STATE_ADD {
                (atoms.to_vec(), vec![])
            } else {
                atoms.iter()
                    .partition::<Vec<Atom>,_>(|x| {
                        !active_atoms.iter().any(|y| y == *x)
                    })
            };

            let atoms: Vec<Atom> = active_atoms.iter()
                .filter(|&x| !rem_atoms.iter().any(|y| y == x))
                .chain(add_atoms.iter())
                .copied()
                .collect();

            xws.change_property(window, "_NET_WM_STATE", XA_ATOM, PropModeReplace, &atoms);

            Some((add_atoms, rem_atoms))
        },
        _ => None
    }
}

pub fn is_window_fullscreen(xws: &XlibWindowSystem, window: Window) -> bool {
    xws.get_property(window, "_NET_WM_STATE")
            .map(|prop| {
                prop.iter()
                    .any(|&x| x == xws.get_atom("_NET_WM_STATE_FULLSCREEN"))
            })
            .unwrap_or(false)
}
