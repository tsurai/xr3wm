#![allow(non_upper_case_globals, unused_variables, dead_code)]
#![allow(clippy::too_many_arguments)]

extern crate libc;

use crate::keycode::{MOD_2, MOD_LOCK};
use crate::layout::Rect;
use crate::ewmh;
use std::{cmp, env, ptr, str};
use std::mem::MaybeUninit;
use std::slice::from_raw_parts;
use std::ffi::{CStr, CString};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use self::libc::{c_void, c_char, c_uchar, c_int, c_uint, c_long, c_ulong};
use self::XlibEvent::*;
use x11::xinerama::XineramaQueryScreens;
use x11::xlib::*;

extern "C" fn error_handler(display: *mut Display, event: *mut XErrorEvent) -> c_int {
    // TODO: proper error handling
    // HACK: fixes LeaveNotify on invalid windows
    0
}

pub enum XlibEvent {
    XMapRequest(Window, bool),
    XClientMessage(Window, Atom, ClientMessageData),
    XConfigureNotify(Window),
    XConfigureRequest(Window, WindowChanges, u32),
    XDestroy(Window),
    XUnmapNotify(Window, bool),
    XPropertyNotify(Window, u64, bool),
    XEnterNotify(Window, bool, u32, u32),
    XFocusIn(Window),
    XFocusOut(Window),
    XKeyPress(Window, u8, String),
    XButtonPress(Window),
    WMClose,
    Ignored,
}

pub struct SizeHint {
    pub min: Option<(u32, u32)>,
    pub max: Option<(u32, u32)>,
}

pub struct Strut(pub u32, pub u32, pub u32, pub u32);

pub struct WindowChanges {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub border_width: u32,
    pub sibling: Window,
    pub stack_mode: u32,
}

pub struct XlibWindowSystem {
    display: *mut Display,
    root: Window,
    event: *mut c_void,
    atoms: HashMap<&'static str, Atom>,
}

impl XlibWindowSystem {
    pub fn new() -> XlibWindowSystem {
        unsafe {
            let display = XOpenDisplay(ptr::null_mut());
            if display.is_null() {
                error!("Can't open display {}",
                       env::var("DISPLAY").unwrap_or_else(|_| "undefined".to_string()));
                panic!();
            }

            let root = XDefaultRootWindow(display);

            XlibWindowSystem {
                display,
                root,
                event: libc::malloc(256),
                atoms: HashMap::new(),
            }
        }
    }

    pub fn init(&mut self) {
        unsafe {
            XSelectInput(self.display, self.root, 0x001A_0034 | EnterWindowMask | PropertyChangeMask);
            XDefineCursor(self.display, self.root, XCreateFontCursor(self.display, 68));
            XSetErrorHandler(Some(error_handler));
            XSync(self.display, 0);
        }

        self.cache_atoms(&["WM_DELETE_WINDOW", "WM_HINTS", "WM_PROTOCOLS", "WM_STATE", "WM_TAKE_FOCUS", "UTF8_STRING"]);
        ewmh::init_ewmh(self);
    }

    pub fn close(&mut self) {
        unsafe {
            XCloseDisplay(self.display);
            self.display = ptr::null_mut();
            libc::free(self.event);
        }
    }

    pub fn get_root_window(&self) -> Window {
        self.root
    }

    pub fn setup_window(&self,
                        x: u32,
                        y: u32,
                        width: u32,
                        height: u32,
                        border_width: u32,
                        border_color: u32,
                        window: Window) {
        self.set_window_border_width(window, border_width);
        self.set_window_border_color(window, border_color);

        self.move_resize_window(window,
                                x,
                                y,
                                cmp::max(width as i32 - (2 * border_width as i32), 0) as u32,
                                cmp::max(height as i32 - (2 * border_width as i32), 0) as u32);
    }

    pub fn create_hidden_window(&self) -> Window {
        unsafe {
            XCreateSimpleWindow(self.display, self.root, -1, -1, 1, 1, 0, 0, 0)
        }
    }

    pub fn get_property<A: IntoAtom>(&self, window: Window, atom: A) -> Option<Vec<u64>> {
        unsafe {
            let mut ret_type: c_ulong = 0;
            let mut ret_format: c_int = 0;
            let mut ret_nitems: c_ulong = 0;
            let mut ret_bytes_after: c_ulong = 0;
            let mut ret_ptr = MaybeUninit::<*mut c_ulong>::uninit();

            if XGetWindowProperty(self.display,
                                  window,
                                  atom.into(self),
                                  0,
                                  0xFFFF_FFFF,
                                  0,
                                  0,
                                  &mut ret_type,
                                  &mut ret_format,
                                  &mut ret_nitems,
                                  &mut ret_bytes_after,
                                  ret_ptr.as_mut_ptr() as *mut *mut c_uchar) == 0 {
                if ret_format != 0 {
                    let ret_ptr = ret_ptr.assume_init();
                    let vec = from_raw_parts(ret_ptr as *const c_ulong, ret_nitems as usize).to_vec();
                    XFree(ret_ptr as *mut c_void);
                    Some(vec)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    pub fn cache_atoms(&mut self, atoms_str: &[&'static str]) {
        let atoms = self.create_atoms(atoms_str);

        for (s,a) in atoms_str.iter().zip(atoms.iter()) {
            self.atoms.insert(s, *a);
        }
    }

    fn create_atom(&self, atom: &str) -> u64 {
        unsafe {
            let cstr = CString::new(atom.as_bytes())
                .unwrap();
            XInternAtom(self.display, cstr.as_ptr() as *mut i8, 0)
        }
    }

    fn create_atoms(&self, atoms: &[&'static str]) -> Vec<u64> {
        let mut atoms: Vec<*mut c_char> = atoms.iter()
            .map(|x| {
                CString::new(x.as_bytes())
                    .unwrap()
                    .into_raw()
            })
            .collect();

        let mut ret_atoms: Vec<u64> = Vec::with_capacity(atoms.len());
        unsafe {
            XInternAtoms(self.display, atoms.as_mut_ptr(), atoms.len() as i32, 0, ret_atoms.as_ptr() as *mut u64);
            ret_atoms.set_len(atoms.len());
        }

        for x in atoms {
            unsafe {
                drop(CString::from_raw(x));
            }
        }

        ret_atoms
    }

    pub fn get_atom(&self, atom: &str) -> u64 {
        self.atoms.get(atom)
            .cloned()
            .unwrap_or_else(|| { trace!("cache miss for: {}", atom); self.create_atom(atom) })
    }

    pub fn get_atom_name(&self, atom: u64) -> String {
        unsafe {
            let ptr = XGetAtomName(self.display, atom);
            let ret = Self::ptr_to_string(ptr);
            XFree(ptr as *mut c_void);
            ret
        }
    }

    pub fn get_windows(&self) -> Vec<Window> {
        unsafe {
            let mut ret_root: c_ulong = 0;
            let mut ret_parent: c_ulong = 0;
            let mut ret_nchildren: c_uint = 0;
            let mut ret_ptr = MaybeUninit::<*mut c_ulong>::uninit();

            XQueryTree(self.display,
                       self.root,
                       &mut ret_root,
                       &mut ret_parent,
                       ret_ptr.as_mut_ptr(),
                       &mut ret_nchildren);

            let ret_ptr = ret_ptr.assume_init();
            let vec = from_raw_parts(ret_ptr, ret_nchildren as usize).to_vec();
            XFree(ret_ptr as *mut c_void);
            vec
        }
    }

    pub fn get_window_strut(&self, window: Window) -> Option<Vec<u64>> {
        let atom = self.get_atom("_NET_WM_STRUT_PARTIAL");
        self.get_property(window, atom)
    }

    pub fn get_window_state(&self, window: Window) -> Option<u8> {
        let atom = self.get_atom("WM_STATE");
        self.get_property(window, atom)
            .and_then(|x| x.first().map(|s| *s as u8))
    }

    pub fn get_window_attributes(&self, window: Window) -> XWindowAttributes {
        unsafe {
            let mut ret_attributes = MaybeUninit::<XWindowAttributes>::zeroed();
            XGetWindowAttributes(self.display, window, ret_attributes.as_mut_ptr());

            ret_attributes.assume_init()
        }
    }

    // TODO: cache result and split into computation and getter functions.
    // Struts rarely change and dont have to be computed on every redraw (see strut layout)
    pub fn compute_struts(&self, screen: Rect) -> Strut {
        self.get_windows()
            .iter()
            .filter_map(|&w| {
                self.get_window_strut(w)
            })
            .filter(|x| {
                let screen_x = u64::from(screen.x);
                let screen_y = u64::from(screen.y);
                let screen_height = u64::from(screen.height);
                let screen_width = u64::from(screen.width);

                (x[0] > 0 &&
                 ((x[4] >= screen_y && x[4] < screen_y + screen_height) ||
                  (x[5] >= screen_y && x[5] <= screen_y + screen_height))) ||
                (x[1] > 0 &&
                 ((x[6] >= screen_y && x[6] < screen_y + screen_height) ||
                  (x[7] >= screen_y && x[7] <= screen_y + screen_height))) ||
                (x[2] > 0 &&
                 ((x[8] >= screen_x && x[8] < screen_x + screen_width) ||
                  (x[9] >= screen_x && x[9] <= screen_x + screen_width))) ||
                (x[3] > 0 &&
                 ((x[10] >= screen_x && x[10] < screen_x + screen_width) ||
                  (x[11] >= screen_x && x[11] <= screen_x + screen_width)))
            })
            .map(|x| Strut(x[0] as u32, x[1] as u32, x[2] as u32, x[3] as u32))
            .fold(Strut(0, 0, 0, 0), |a, b| {
                Strut(cmp::max(a.0, b.0),
                      cmp::max(a.1, b.1),
                      cmp::max(a.2, b.2),
                      cmp::max(a.3, b.3))
            })
    }

    pub fn change_property<T: Into<u64>, A: IntoAtom>(&self,
                       window: Window,
                       atom: &str,
                       atom_type: A,
                       mode: c_int,
                       data: &[T])
    {
        unsafe {
            XChangeProperty(self.display,
                            window,
                            self.get_atom(atom),
                            atom_type.into(self),
                            // Xlib requires char for format 8, short for 16 and 32 for long
                            // skipping over int32 for some reason
                            std::cmp::min(32, std::mem::size_of::<T>() as i32 * 8),
                            mode,
                            data.as_ptr().cast::<u8>(),
                            data.len() as i32);
        }
    }

    pub fn send_client_message<T: Into<ClientMessageData>>(&self, window: Window, atom: &str, data: T) {
        let mut event = XClientMessageEvent {
            type_: ClientMessage,
            serial: 0,
            send_event: 0,
            display: ptr::null_mut(),
            window,
            message_type: self.get_atom(atom) as c_ulong,
            format: 32,
            data: data.into(),
        };

        let event_ptr: *mut XClientMessageEvent = &mut event;
        unsafe {
            XSendEvent(self.display, self.root, 0, NoEventMask, event_ptr as *mut XEvent);
        }
    }

    pub fn delete_property<A: IntoAtom>(&self, window: Window, atom: A) {
        unsafe {
            XDeleteProperty(self.display, window, atom.into(self));
        }
    }

    pub fn configure_window(&self,
                            window: Window,
                            window_changes: WindowChanges,
                            mask: u32,
                            floating: bool) {
        unsafe {
            if floating {
                let mut ret_window_changes = XWindowChanges {
                    x: window_changes.x as i32,
                    y: window_changes.y as i32,
                    width: window_changes.width as i32,
                    height: window_changes.height as i32,
                    border_width: window_changes.border_width as i32,
                    sibling: window_changes.sibling,
                    stack_mode: window_changes.stack_mode as i32,
                };
                XConfigureWindow(self.display, window, mask, &mut ret_window_changes);
            } else {
                let rect = self.get_geometry(window);
                let mut attributes = MaybeUninit::zeroed();

                XGetWindowAttributes(self.display, window, attributes.as_mut_ptr());

                let mut event = XConfigureEvent {
                    type_: ConfigureNotify,
                    display: self.display,
                    serial: 0,
                    send_event: 1,
                    x: rect.x as i32,
                    y: rect.y as i32,
                    width: rect.width as i32,
                    height: rect.height as i32,
                    border_width: 0,
                    event: window,
                    window,
                    above: 0,
                    override_redirect: attributes.assume_init().override_redirect,
                };
                let event_ptr: *mut XConfigureEvent = &mut event;
                XSendEvent(self.display, window, 0, 0, event_ptr as *mut XEvent);
            }
            XSync(self.display, 0);
        }
    }

    pub fn show_window(&self, window: Window) {
        unsafe {
            self.change_property(window, "WM_STATE", "WM_STATE", PropModeReplace, &[1u64, 0]);
            XMapWindow(self.display, window);
        }
    }

    pub fn hide_window(&self, window: Window) {
        unsafe {
            XSelectInput(self.display, window, 0x0040_0010 | FocusChangeMask);
            XUnmapWindow(self.display, window);
            XSelectInput(self.display, window, 0x0042_0010 | FocusChangeMask);

            self.change_property(window, "WM_STATE", "WM_STATE", PropModeReplace, &[0u64, 0]);
            self.delete_property(window, "_NET_WM_DESKTOP");
        }
    }

    pub fn lower_window(&self, window: Window) {
        unsafe {
            XLowerWindow(self.display, window);
        }
    }

    pub fn raise_window(&self, window: Window) {
        unsafe {
            XRaiseWindow(self.display, window);
        }
    }

    pub fn unmap_window(&self, window: Window) {
        unsafe {
            XUnmapWindow(self.display, window);
        }
    }

    pub fn move_resize_window(&self, window: Window, x: u32, y: u32, width: u32, height: u32) {
        unsafe {
            XMoveResizeWindow(self.display, window, x as i32, y as i32, width, height);
        }
    }

    pub fn focus_window(&self, window: Window) {
        let input_hint = self.get_wm_hints(window).map(|x| x.input != 0).unwrap_or(true);
        let takes_focus = self.has_protocol(window, "WM_TAKE_FOCUS");

        if input_hint {
            trace!("set input focus via hint");
            unsafe {
                XSetInputFocus(self.display, window, 1, 0);
                self.skip_enter_events();
            }

            ewmh::set_active_window(self, window);
            let state = self.get_atom("_NET_WM_STATE_DEMANDS_ATTENTION");
            self.send_client_message(window, "_NET_WM_STATE", [0, state, 0, 0, 0]);
        } else if takes_focus {
            trace!("send WM_TAKE_FOCUS to: {:#x}", window);
            let time = SystemTime::now().duration_since(UNIX_EPOCH)
                .map(|x| x.as_secs())
                .unwrap_or(0);

            let atom = self.get_atom("WM_TAKE_FOCUS");
            let mut event = XClientMessageEvent {
                type_: ClientMessage,
                serial: 0,
                send_event: 0,
                display: ptr::null_mut(),
                window,
                message_type: self.get_atom("WM_PROTOCOLS") as c_ulong,
                format: 32,
                data: ClientMessageData::from([atom, time, 0, 0, 0]),
            };

            let event_ptr: *mut XClientMessageEvent = &mut event;
            unsafe {
                XSendEvent(self.display, window, 0, NoEventMask, event_ptr as *mut XEvent);
            }
            let state = self.get_atom("_NET_WM_STATE_DEMANDS_ATTENTION");

            ewmh::set_active_window(self, window);
            self.send_client_message(window, "_NET_WM_STATE", [0, state, 0, 0, 0]);
        }
    }

    pub fn skip_enter_events(&self) {
        unsafe {
            let event: *mut c_void = libc::malloc(256);
            XSync(self.display, 0);
            while XCheckMaskEvent(self.display, 16, event as *mut XEvent) != 0 {
            }
            libc::free(event);
        }
    }

    fn has_protocol(&self, window: Window, protocol: &str) -> bool {
        unsafe {
            let mut count: MaybeUninit<c_int> = MaybeUninit::uninit();
            let mut atoms: MaybeUninit<*mut Atom> = MaybeUninit::uninit();

            if XGetWMProtocols(self.display, window, atoms.as_mut_ptr(), count.as_mut_ptr()) != 0 {
                let atoms = atoms.assume_init();
                let count = count.assume_init();
                let protocol_atom = self.get_atom(protocol);

                if count != 0 {
                    let ret = from_raw_parts(atoms, count as usize)
                        .contains(&protocol_atom);

                    XFree(atoms as *mut c_void);

                    return ret
                }
            }
            false
        }
    }

    pub fn kill_window(&self, window: Window) {
        if window == 0 {
            return;
        }

        unsafe {
            if self.has_protocol(window, "WM_DELETE_WINDOW") {
                let time = SystemTime::now().duration_since(UNIX_EPOCH)
                    .map(|x| x.as_secs())
                    .unwrap_or(0);
                let delete_atom = self.get_atom("WM_DELETE_WINDOW");

                let mut event = XClientMessageEvent {
                    type_: ClientMessage,
                    serial: 0,
                    send_event: 0,
                    display: ptr::null_mut(),
                    window,
                    message_type: self.get_atom("WM_PROTOCOLS") as c_ulong,
                    format: 32,
                    data: ClientMessageData::from([delete_atom, time, 0, 0, 0]),
                };

                let event_ptr: *mut XClientMessageEvent = &mut event;
                XSendEvent(self.display, window, 0, NoEventMask, event_ptr as *mut XEvent);
            } else {
                XKillClient(self.display, window);
                XSync(self.display, 0);
            }
        }
    }

    pub fn restack_windows(&self, mut windows: Vec<Window>) {
        unsafe {
            XRestackWindows(self.display,
                            (windows[..]).as_mut_ptr(),
                            windows.len() as i32);
        }
    }

    pub fn grab_button(&self, window: Window) {
        unsafe {
            XGrabButton(self.display, 1, 0x8000, window, 1, 256, 0, 0, 0, 0);
        }
    }

    pub fn grab_modifier(&self, mod_key: u8) {
        unsafe {
            XGrabKey(self.display, 0, u32::from(mod_key), self.root, 1, 0, 1);
            XGrabKey(self.display,
                     0,
                     u32::from(mod_key | MOD_2),
                     self.root,
                     1,
                     0,
                     1);
            XGrabKey(self.display,
                     0,
                     u32::from(mod_key | MOD_LOCK),
                     self.root,
                     1,
                     0,
                     1);
            XGrabKey(self.display,
                     0,
                     u32::from(mod_key | MOD_2 | MOD_LOCK),
                     self.root,
                     1,
                     0,
                     1);
        }
    }

    pub fn keycode_to_string(&self, keycode: u32) -> String {
        unsafe {
            let keysym = XKeycodeToKeysym(self.display, keycode as u8, 0);
            str::from_utf8(CStr::from_ptr(XKeysymToString(keysym) as *const i8).to_bytes())
                .unwrap()
                .to_string()
        }
    }

    pub fn set_window_border_width(&self, window: Window, width: u32) {
        if window != self.root {
            unsafe {
                XSetWindowBorderWidth(self.display, window, width);
            }
        }
    }

    pub fn set_window_border_color(&self, window: Window, color: u32) {
        if window != self.root {
            unsafe {
                XSetWindowBorder(self.display, window, u64::from(color));
            }
        }
    }

    pub fn get_display_width(&self, screen: u32) -> u32 {
        unsafe { XDisplayWidth(self.display, screen as i32) as u32 }
    }

    pub fn get_display_height(&self, screen: u32) -> u32 {
        unsafe { XDisplayHeight(self.display, screen as i32) as u32 }
    }

    pub fn get_display_rect(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: self.get_display_width(0),
            height: self.get_display_height(0),
        }
    }

    pub fn get_geometry(&self, window: Window) -> Rect {
        unsafe {
            let mut root = MaybeUninit::uninit();
            let mut x = MaybeUninit::uninit();
            let mut y = MaybeUninit::uninit();
            let mut width = MaybeUninit::uninit();
            let mut height = MaybeUninit::uninit();
            let mut depth = MaybeUninit::uninit();
            let mut border = MaybeUninit::uninit();

            XGetGeometry(self.display,
                         window,
                         root.as_mut_ptr(),
                         x.as_mut_ptr(),
                         y.as_mut_ptr(),
                         width.as_mut_ptr(),
                         height.as_mut_ptr(),
                         border.as_mut_ptr(),
                         depth.as_mut_ptr());

            Rect {
                x: x.assume_init() as u32,
                y: y.assume_init() as u32,
                width: width.assume_init(),
                height: height.assume_init(),
            }
        }
    }

    pub fn get_screen_infos(&self) -> Vec<Rect> {
        unsafe {
            let mut num: c_int = 0;
            let screen_ptr = XineramaQueryScreens(self.display, &mut num);

            if num == 0 {
                return vec![self.get_display_rect()];
            }

            let vec = from_raw_parts(screen_ptr, num as usize)
                .iter()
                .map(|screen_info| {
                    Rect {
                        x: screen_info.x_org as u32,
                        y: screen_info.y_org as u32,
                        width: screen_info.width as u32,
                        height: screen_info.height as u32,
                    }
                })
                .collect();
            XFree(screen_ptr as *mut c_void);
            vec
        }
    }

    pub fn is_floating_window(&self, window: Window) -> bool {
        if self.transient_for(window).is_some() {
            return true;
        }

        let hints = self.get_size_hints(window);
        let min = hints.min;
        let max = hints.max;

        if min.is_some() && max.is_some() && min.unwrap().0 == max.unwrap().0 &&
           min.unwrap().1 == max.unwrap().1 {
            return true;
        }

        if let Some(property) = self.get_property(window, self.get_atom("_NET_WM_WINDOW_TYPE")) {
            let dialog = self.get_atom("_NET_WM_WINDOW_TYPE_DIALOG");
            let splash = self.get_atom("_NET_WM_WINDOW_TYPE_SPLASH");

            property.iter().any(|&x| x == dialog || x == splash)
        } else {
            false
        }
    }

    pub fn transient_for(&self, window: Window) -> Option<Window> {
        unsafe {
            let mut w = MaybeUninit::uninit();

            if XGetTransientForHint(self.display, window, w.as_mut_ptr()) != 0 {
                Some(w.assume_init())
            } else {
                None
            }
        }
    }

    pub fn get_size_hints(&self, window: Window) -> SizeHint {
        unsafe {
            let mut size_hint = MaybeUninit::zeroed();
            let mut tmp: c_long = 0;
            XGetWMNormalHints(self.display, window, size_hint.as_mut_ptr(), &mut tmp);

            let size_hint = size_hint.assume_init();
            let min = if (size_hint.flags & PMinSize) != 0 {
                Some((size_hint.min_width as u32, size_hint.min_height as u32))
            } else {
                None
            };

            let max = if (size_hint.flags & PMaxSize) != 0 {
                Some((size_hint.max_width as u32, size_hint.max_height as u32))
            } else {
                None
            };
            SizeHint {
                min,
                max,
            }
        }
    }

    pub fn get_wm_hints(&self, window: Window) -> Option<XWMHints> {
        unsafe {
            let ptr = XGetWMHints(self.display, window);
            let ret = ptr.as_ref().copied();
            XFree(ptr as *mut c_void);
            ret
        }
    }

    pub fn is_urgent(&self, window: Window) -> bool {
        self.get_wm_hints(window)
            .map(|x| x.flags & XUrgencyHint != 0)
            .unwrap_or(false)
    }

    pub fn get_class_name(&self, window: Window) -> Option<String> {
        unsafe {
            let mut hint = MaybeUninit::uninit();

            if XGetClassHint(self.display, window, hint.as_mut_ptr()) != 0 {
                let hint = hint.assume_init();
                if !hint.res_class.is_null() {
                    let hint_cstr = CStr::from_ptr(hint.res_class);
                    let ret = str::from_utf8(hint_cstr.to_bytes())
                        .map(|x| x.to_owned())
                        .ok();
                    XFree(hint.res_class as *mut c_void);
                    XFree(hint.res_name as *mut c_void);
                    return ret;
                }
            }
            None
        }
    }

    pub fn get_window_title(&self, window: Window) -> String {
        if window == self.root {
            return String::new();
        }

        unsafe {
            let mut name = MaybeUninit::uninit();

            let ret = XGetTextProperty(self.display, window, name.as_mut_ptr(), self.get_atom("_NET_WM_NAME"));

            let ptr = if ret != 0 {
                name.assume_init().value as *const i8
            } else {
                let mut name = MaybeUninit::uninit();

                if XFetchName(self.display, window, name.as_mut_ptr()) != 0 {
                    name.assume_init() as *const i8
                } else {
                    ptr::null()
                }
            };

            if !ptr.is_null() {
                let ret_str = Self::ptr_to_string(ptr);
                XFree(ptr as *mut c_void);
                ret_str
            } else {
                String::new()
            }
        }
    }

    unsafe fn ptr_to_string(ptr: *const i8) -> String {
        match str::from_utf8(CStr::from_ptr(ptr).to_bytes()) {
            Ok(s) => s.to_string(),
            Err(_) => String::new(),
        }
    }

    pub fn move_pointer(&self, x: i32, y: i32) {
        unsafe {
            let mut root_w = MaybeUninit::uninit();
            let mut child_w = MaybeUninit::uninit();
            let mut root_x = MaybeUninit::uninit();
            let mut root_y = MaybeUninit::uninit();
            let mut win_x = MaybeUninit::uninit();
            let mut win_y = MaybeUninit::uninit();
            let mut mask = MaybeUninit::uninit();

            let ret = XQueryPointer(
                self.display,
                self.root,
                root_w.as_mut_ptr() as *mut Window,
                child_w.as_mut_ptr() as *mut Window,
                root_x.as_mut_ptr(),
                root_y.as_mut_ptr(),
                win_x.as_mut_ptr(),
                win_y.as_mut_ptr(),
                mask.as_mut_ptr());

            if ret == 1 {
                XWarpPointer(self.display, 0, 0, 0, 0, 0, 0, x - root_x.assume_init(), y - root_y.assume_init());
            }
        }
    }

    pub fn request_window_events(&self, window: Window) {
        unsafe {
            self.grab_button(window);
            XSelectInput(self.display, window, 0x0042_0010 | FocusChangeMask);
        }
    }

    fn cast_event_to<T>(&self) -> &T {
        unsafe { &*(self.event as *const T) }
    }

    #[allow(clippy::nonminimal_bool)]
    pub fn get_event(&self) -> XlibEvent {
        if self.display.is_null() {
            return WMClose;
        }

        unsafe {
            XNextEvent(self.display, self.event as *mut XEvent);
        }

        let evt_type: c_int = *self.cast_event_to();
        match evt_type {
            /*
             * MapRequest is triggered whenever a client initiates a map window request on an unmapped window
             * whose override_redirect is set to false
             */
            MapRequest => {
                let evt: &XMapRequestEvent = self.cast_event_to();
                trace!("MapRequest {:?}", evt);

                // Some docks rely entirely on the EWMH window type and do not set redirect
                // override to prevent the WM from reparenting it
                let dock_type = self.get_atom("_NET_WM_WINDOW_TYPE_DOCK");
                let is_dock = self.get_property(evt.window, "_NET_WM_WINDOW_TYPE")
                    .map(|atoms| atoms.iter().any(|&a| a == dock_type))
                    .unwrap_or(false);

                let sticky_type = self.get_atom("_NET_WM_STATE_STICKY");
                let is_sticky = self.get_property(evt.window, "_NET_WM_STATE")
                    .map(|atoms| atoms.iter().any(|&a| a == sticky_type))
                    .unwrap_or(false);

                if is_dock {
                    // map the dock but do not manage it any further
                    unsafe {
                        XMapWindow(self.display, evt.window);
                    }
                    Ignored
                } else {
                    self.request_window_events(evt.window);

                    XMapRequest(evt.window, is_sticky)
                }
            }
            /*
             * MapNotify is triggered whenever a client is mapped regardless of the value of override_redirect.
             * Compard to MapRequest this also catches windows that are not suppose to be managed
             * by the WM.
             */
            MapNotify => {
                let evt: &XMapEvent = self.cast_event_to();
                trace!("MapNotify: {:?}", evt);

                // only windows with override redirect dont have WM_STATE
                if self.get_property(evt.window, "WM_STATE").is_none() {
                    unsafe {
                        XSelectInput(self.display, evt.window, PropertyChangeMask);
                    }
                }
                Ignored
            }
            ClientMessage => {
                let evt: &XClientMessageEvent = self.cast_event_to();
                XClientMessage(evt.window, evt.message_type, evt.data)
            }
            ConfigureNotify => {
                let evt: &XConfigureEvent = self.cast_event_to();
                if evt.window == self.root {
                    XConfigureNotify(evt.window)
                } else {
                    Ignored
                }
            }
            ConfigureRequest => {
                let event: &XConfigureRequestEvent = self.cast_event_to();
                let changes = WindowChanges {
                    x: event.x as u32,
                    y: event.y as u32,
                    width: event.width as u32,
                    height: event.height as u32,
                    border_width: event.border_width as u32,
                    sibling: event.above as Window,
                    stack_mode: event.detail as u32,
                };
                XConfigureRequest(event.window, changes, event.value_mask as u32)
            }
            DestroyNotify => {
                let evt: &XDestroyWindowEvent = self.cast_event_to();
                XDestroy(evt.window)
            }
            UnmapNotify => {
                let evt: &XUnmapEvent = self.cast_event_to();
                XUnmapNotify(evt.window, evt.send_event > 0)
            }
            PropertyNotify => {
                let evt: &XPropertyEvent = self.cast_event_to();
                XPropertyNotify(evt.window, evt.atom, evt.state == 0)
            }
            EnterNotify => {
                let evt: &XEnterWindowEvent = self.cast_event_to();
                if evt.detail != 2 {
                    XEnterNotify(evt.window, false, evt.x as u32, evt.y as u32)
                } else if evt.detail == 2 && evt.window == self.root {
                    XEnterNotify(evt.window, true, evt.x as u32, evt.y as u32)
                } else {
                    Ignored
                }
            }
            FocusIn => {
                let evt: &XFocusInEvent = self.cast_event_to();
                trace!("XFocusIn: {:?}", evt);
                if (evt.mode == NotifyNormal &&
                        // mouse focus move from root to window
                        (evt.detail == NotifyAncestor ||
                        // mouse focus move from window to window
                        evt.detail == NotifyNonlinear)) ||
                    (evt.mode == NotifyWhileGrabbed &&
                        // manual focus move from root to window
                        (evt.detail == NotifyAncestor ||
                        // manual focus move from window to window
                        evt.detail == NotifyNonlinear))
                {
                    XFocusIn(evt.window)
                } else {
                    Ignored
                }
            }
            ButtonPress => {
                let evt: &XButtonPressedEvent = self.cast_event_to();
                unsafe {
                    XAllowEvents(self.display, 2, 0);
                }

                XButtonPress(evt.window)
            }
            KeyPress => {
                let evt: &XKeyPressedEvent = self.cast_event_to();
                XKeyPress(evt.window,
                          evt.state as u8,
                          self.keycode_to_string(evt.keycode))
            }
            _ => Ignored,
        }
    }
}

impl Default for XlibWindowSystem {
    fn default() -> Self {
        Self::new()
    }
}

pub trait IntoAtom {
    fn into(self, xws: &XlibWindowSystem) -> Atom;
}

impl IntoAtom for Atom {
    fn into(self, _: &XlibWindowSystem) -> Atom {
        self
    }
}

impl IntoAtom for &str {
    fn into(self, xws: &XlibWindowSystem) -> Atom {
        xws.get_atom(self)
    }
}
