use crate::xlib_window_system::XlibWindowSystem;
use crate::workspace::Workspace;
use x11::xlib::*;
use std::ffi::CString;

// TODO: cache atoms

pub fn init_ewmh(xws: &XlibWindowSystem, root: Window) {
    debug!("initializing ewmh");

    xws.change_property(root, "_NET_SUPPORTED", XA_ATOM, PropModeReplace, &[
        xws.get_atom("_NET_SUPPORTED", true),
        xws.get_atom("_NET_SUPPORTING_WM_CHECK", true),
        xws.get_atom("_NET_NUMBER_OF_DESKTOPS", true),
        xws.get_atom("_NET_DESKTOP_NAMES", true),
        xws.get_atom("_NET_DESKTOP_VIEWPORT", true),
        xws.get_atom("_NET_CURRENT_DESKTOP", true),
        xws.get_atom("_NET_WM_STRUT_PARTIAL", true),
        xws.get_atom("_NET_ACTIVE_WINDOW", true),
        xws.get_atom("_NET_CLIENT_LIST", true),
    ]);

    let window = xws.create_hidden_window();
    xws.change_property(root, "_NET_SUPPORTING_WM_CHECK", XA_WINDOW, PropModeReplace, &[window]);
    xws.change_property(window, "_NET_SUPPORTING_WM_CHECK", XA_WINDOW, PropModeReplace, &[window]);

    let wm_name = CString::new("xr3wm").unwrap();
    xws.change_property(window, "_NET_WM_NAME", "UTF8_STREAM", PropModeReplace, wm_name.as_bytes_with_nul());
}

pub fn set_active_window(xws: &XlibWindowSystem, window: Window) {
    let root = xws.get_root_window();
    xws.change_property(root, "_NET_ACTIVE_WINDOW", XA_WINDOW, PropModeReplace, &[window]);
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

pub fn set_window_fullscreen(xws: &XlibWindowSystem, window: Window, is_fullscreen: bool) {
    if is_fullscreen {
        xws.change_property(window, "_NET_WM_STATE", XA_ATOM, PropModeReplace, &[
            xws.get_atom("_NET_WM_STATE_FULLSCREEN", true)
        ]);
    } else {
        xws.delete_property(window, "_NET_WM_STATE");
    }
}

pub fn is_window_fullscreen(xws: &XlibWindowSystem, window: Window) -> bool {
    xws.get_property(window, "_NET_WM_STATE")
            .map(|prop| {
                prop.first()
                    .map(|&x| x == xws.get_atom("_NET_WM_STATE_FULLSCREEN", true))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
}
