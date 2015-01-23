#![allow(non_upper_case_globals, unused_variables)]

extern crate libc;

use keycode::{MOD_2, MOD_LOCK};
use layout::Rect;
use std::cmp;
use std::str;
use std::fmt;
use std::os::env;
use std::str::from_c_str;
use std::ptr::null_mut;
use std::mem::{uninitialized, transmute};
use std::slice::from_raw_buf;
use std::ffi::{CString, c_str_to_bytes};
use self::libc::{c_void, c_int, c_uint, c_char, c_uchar, c_long, c_ulong};
use self::libc::funcs::c95::stdlib::malloc;
use self::XlibEvent::*;
use xinerama::XineramaQueryScreens;
use xlib::*;

extern fn error_handler(display: *mut Display, event: *mut XErrorEvent) -> c_int {
  // TODO: proper error handling
  // HACK: fixes LeaveNotify on invalid windows
  return 0;
}

const KeyPress             : i32 = 2;
const ButtonPress          : i32 = 4;
const EnterNotify          : i32 = 7;
const FocusOut             : i32 = 10;
const Destroy              : i32 = 17;
const UnmapNotify          : i32 = 18;
const MapRequest           : i32 = 20;
const ConfigurationNotify  : i32 = 22;
const ConfigurationRequest : i32 = 23;
const PropertyNotify       : i32 = 28;

const Success     : i32 = 0;
const BadRequest  : i32 = 1;
const BadValue    : i32 = 2;
const BadWindow   : i32 = 3;
const BadPixmap   : i32 = 4;
const BadAtom     : i32 = 5;
const BadCursor   : i32 = 6;
const BadFont     : i32 = 7;
const BadMatch    : i32 = 8;
const BadDrawable : i32 = 9;
const BadAccess   : i32 = 10;

pub struct XlibWindowSystem {
  display:   *mut Display,
  root:      Window,
  event:     *mut c_void
}

pub enum XlibEvent {
  XMapRequest(Window),
  XConfigurationNotify(Window),
  XConfigurationRequest(Window, WindowChanges, u32),
  XDestroy(Window),
  XUnmapNotify(Window, bool),
  XPropertyNotify(Window, u64, bool),
  XEnterNotify(Window),
  XFocusOut(Window),
  XKeyPress(Window, u8, String),
  XButtonPress(Window),
  Ignored
}

pub struct SizeHint {
  pub min: Option<(u32,u32)>,
  pub max: Option<(u32,u32)>
}

pub struct Strut(pub u32, pub u32, pub u32, pub u32);

pub struct WindowChanges {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
  pub border_width: u32,
  pub sibling: Window,
  pub stack_mode: u32
}

impl fmt::String for WindowChanges {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{{ x: {}, y: {}, width: {}, height: {}, border_width: {}, sibling: {}, stack_mode: {} }}", 
      self.x,
      self.y,
      self.width,
      self.height,
      self.border_width,
      self.sibling,
      self.stack_mode)
  }
}

impl XlibWindowSystem {
  pub fn new() -> XlibWindowSystem {
    unsafe {
      let display = XOpenDisplay(null_mut());
      if display.is_null() {
        error!("Can't open display{}", env().iter().find(|&&(ref x,_)| *x == String::from_str("DISPLAY")).map(|&(_,ref x)| x.clone()).unwrap());
        panic!();
      }

      let root = XDefaultRootWindow(display);
      XSelectInput(display, root, 0x1A0034);
      XDefineCursor(display, root, XCreateFontCursor(display, 68));
      XSetErrorHandler(error_handler as *mut u8);

      XlibWindowSystem {
        display: display,
        root: root,
        event: malloc(256)
      }
    }
  }

  pub fn close(&self) {
    unsafe {
      XCloseDisplay(self.display);
    }
  }

  pub fn setup_window(&self, x: u32, y: u32, width: u32, height: u32, border_width: u32, border_color: u32, window: Window) {
    self.set_window_border_width(window, border_width);
    self.set_window_border_color(window, border_color);
    self.move_resize_window(window, x, y, width - (2 * border_width), height - (2 * border_width));
  }

  fn get_property(&self, window: Window, property: u64) -> Option<Vec<u64>> {
    unsafe {
      let mut ret_type : c_ulong = 0;
      let mut ret_format : c_int = 0;
      let mut ret_nitems : c_ulong = 0;
      let mut ret_bytes_after : c_ulong = 0;
      let mut ret_prop : *mut c_uchar = uninitialized();

      if XGetWindowProperty(self.display, window, property, 0, 0xFFFFFFFF, 0, 0, &mut ret_type, &mut ret_format, &mut ret_nitems, &mut ret_bytes_after, &mut ret_prop) == Success {
        if ret_format != 0 {
          Some(from_raw_buf(&(ret_prop as *const c_ulong), ret_nitems as usize).iter().map(|&x| x as u64).collect())
        } else {
          None
        }
      } else {
        None
      }
    }
  }

  pub fn get_atom(&self, s: &str) -> u64 {
    unsafe {
      XInternAtom(self.display, CString::from_slice(s.as_bytes()).as_slice_with_nul().as_ptr() as *mut i8, 0) as u64
    }
  }

  fn get_windows(&self) -> Vec<Window> {
    unsafe {
      let mut ret_root : c_ulong = 0;
      let mut ret_parent : c_ulong = 0;
      let mut ret_nchildren : c_uint = 0;
      let mut ret_children : *mut c_ulong = uninitialized();

      XQueryTree(self.display, self.root, &mut ret_root, &mut ret_parent, &mut ret_children, &mut ret_nchildren);
      from_raw_buf(&(ret_children as *const c_ulong), ret_nchildren as usize).iter().map(|&x| x as u64).collect()
    }
  }

  pub fn get_strut(&self, screen: Rect) -> Strut {
    let atom = self.get_atom("_NET_WM_STRUT_PARTIAL");

    self.get_windows().iter().filter_map(|&w| self.get_property(w, atom)).filter(|x| {
      match x.as_slice() {
        [ls, rs, ts, bs, l1, l2, r1, r2, t1, t2, b1, b2] => {
          ((ls > 0 || rs > 0) && (l1 >= screen.y as u64 && l1 <= screen.height as u64) || (l2 >= screen.y as u64 && l2 <= screen.height as u64)) ||
          ((ts > 0 || bs > 0) && (t1 >= screen.x as u64 && t1 <= screen.width as u64)  || (t2 >= screen.x as u64 && t2 <= screen.width as u64))
        },
        _ => { false }
      }
    }).map(|x| Strut(x[0] as u32, x[1] as u32, x[2] as u32, x[3] as u32)).fold(Strut(0, 0, 0, 0), |a, b| Strut(cmp::max(a.0, b.0), cmp::max(a.1, b.1), cmp::max(a.2, b.2), cmp::max(a.3, b.3)))
  }

  fn change_property(&self, window: Window, property: u64, typ: u64, mode: c_int, dat: &mut [c_ulong]) {
    unsafe {
      let ptr : *mut u8 = transmute(dat.as_mut_ptr());
      XChangeProperty(self.display, window, property as c_ulong, typ as c_ulong, 32, mode, ptr, 2);
    }
  }

  pub fn configure_window(&self, window: Window, window_changes: WindowChanges, mask: u32, unmanaged: bool) {
    unsafe {
      if unmanaged {
        let mut ret_window_changes = XWindowChanges{
          x: window_changes.x as i32,
          y: window_changes.y as i32,
          width: window_changes.width as i32,
          height: window_changes.height as i32,
          border_width: window_changes.border_width as i32,
          sibling: window_changes.sibling,
          stack_mode: window_changes.stack_mode as i32
        };
        XConfigureWindow(self.display, window, mask, &mut ret_window_changes);
      } else {
        let rect = self.get_geometry(window);
        let mut attributes : XWindowAttributes = uninitialized();
        XGetWindowAttributes(self.display, window, &mut attributes);
        let mut event = XConfigureEvent {
          _type: ConfigurationRequest as i32,
          display: self.display,
          serial: 0,
          send_event: 1,
          x: rect.x as i32,
          y: rect.y as i32,
          width: rect.width as i32,
          height: rect.height as i32,
          border_width: 0,
          event: window,
          window: window,
          above: 0,
          override_redirect: attributes.override_redirect
        };
        let event_ptr : *mut XConfigureEvent = &mut event;
        XSendEvent(self.display, window, 0, 0, (event_ptr as *mut c_void));
      }
      XSync(self.display, 0);
    }
  }

  pub fn show_window(&self, window: Window) {
    unsafe {
      let atom = self.get_atom("WM_STATE");
      self.change_property(window, atom, atom, 0, &mut [1, 0]);
      XMapWindow(self.display, window);
    }
  }

  pub fn hide_window(&self, window: Window) {
    unsafe {
      XSelectInput(self.display, window, 0x400010);
      XUnmapWindow(self.display, window);
      XSelectInput(self.display, window, 0x420010);

      let atom = self.get_atom("WM_STATE");
      self.change_property(window as u64, atom, atom, 0, &mut [3, 0]);
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

  pub fn focus_window(&self, window: Window, color: u32) {
    unsafe {
      XSetInputFocus(self.display, window, 1, 0);
      self.set_window_border_color(window, color);
      XSync(self.display, 0);
    }
  }

  pub fn skip_enter_events(&self) {
    unsafe {
      let event : *mut c_void = malloc(256);
      XSync(self.display, 0);
      while XCheckMaskEvent(self.display, 16, event) != 0 { }
    }
  }

  fn has_protocol(&self, window: Window, protocol: &str) -> bool {
    unsafe {
      let mut count : c_int = uninitialized();
      let mut atoms : *mut Atom = uninitialized();

      XGetWMProtocols(self.display, window, &mut atoms, &mut count);
      from_raw_buf(&(atoms as *const c_ulong), count as usize).contains(&self.get_atom(protocol))
    }
  }

  pub fn kill_window(&self, window: Window) {
    if window == 0 {
      return;
    }

    unsafe {
      if self.has_protocol(window, "WM_DELETE_WINDOW") {
        let event = XClientMessageEvent {
          serial: 0,
          send_event: 0,
          _type: 33,
          format: 32,
          display: self.display,
          window: window,
          message_type: self.get_atom("WM_PROTOCOLS") as c_ulong,
          data: [((self.get_atom("WM_DELETE_WINDOW") & 0xFFFFFFFF00000000) >> 32) as i32,
                 (self.get_atom("WM_DELETE_WINDOW") & 0xFFFFFFFF) as i32, 0, 0, 0]
        };

        XSendEvent(self.display, window, 0, 0, transmute(&event));
      } else {
        XKillClient(self.display, window);
      }
    }
  }

  pub fn restack_windows(&self, mut windows: Vec<Window>) {
    unsafe {
      for w in windows.iter() {
        debug!("{}", w);
      }
      XRestackWindows(self.display, windows.as_mut_slice().as_mut_ptr(), windows.len() as i32);
    }
  }

  pub fn grab_button(&self, window: Window) {
    unsafe {
      XGrabButton(self.display, 1, 0x8000, window, 1, 256, 0, 0, 0, 0);
    }
  }

  pub fn grab_modifier(&self, mod_key: u8) {
    unsafe {
      XGrabKey(self.display, 0, mod_key as u32, self.root, 1, 0, 1);
      XGrabKey(self.display, 0, (mod_key | MOD_2) as u32, self.root, 1, 0, 1);
      XGrabKey(self.display, 0, (mod_key | MOD_LOCK) as u32, self.root, 1, 0, 1);
      XGrabKey(self.display, 0, (mod_key | MOD_2 | MOD_LOCK) as u32, self.root, 1, 0, 1);
    }
  }

  pub fn keycode_to_string(&self, keycode: u32) -> String {
    unsafe {
      let keysym = XKeycodeToKeysym(self.display, keycode as u8, 0);
      String::from_str(str::from_c_str(transmute(XKeysymToString(keysym))))
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
        XSetWindowBorder(self.display, window, color as c_ulong);
      }
    }
  }

  pub fn get_display_width(&self, screen: u32) -> u32 {
    unsafe {
      XDisplayWidth(self.display, screen as i32) as u32
    }
  }

  pub fn get_display_height(&self, screen: u32) -> u32 {
    unsafe {
      XDisplayHeight(self.display, screen as i32) as u32
    }
  }

  pub fn get_display_rect(&self) -> Rect {
    Rect {
      x: 0,
      y: 0,
      width: self.get_display_width(0),
      height: self.get_display_height(0)
    }
  }

  pub fn get_geometry(&self, window: Window) -> Rect {
    unsafe {
      let mut root : Window = uninitialized();
      let mut x : c_int = uninitialized();
      let mut y : c_int = uninitialized();
      let mut width : c_uint = uninitialized();
      let mut height : c_uint = uninitialized();
      let mut depth : c_uint = uninitialized();
      let mut border : c_uint = uninitialized();

      XGetGeometry(self.display, window, &mut root, &mut x, &mut y, &mut width, &mut height, &mut border, &mut depth);

      Rect {
        x: x as u32,
        y: y as u32,
        width: width,
        height: height
      }
    }
  }

  pub fn get_screen_infos(&self) -> Vec<Rect> {
    unsafe {
      let mut num : c_int = 0;
      let screen_ptr = XineramaQueryScreens(self.display, &mut num);

      if num == 0 {
        return vec!(self.get_display_rect());
      }

      from_raw_buf(&screen_ptr, num as usize).iter().map(|ref screen_info|
        Rect {
          x: screen_info.x_org as u32,
          y: screen_info.y_org as u32,
          width: screen_info.width as u32,
          height: screen_info.height as u32
        }
      ).collect()
    }
  }

  pub fn is_window_floating(&self, window: Window) -> bool {
    if self.transient_for(window).is_some() {
      return true;
    }

    let hints = self.get_size_hints(window);
    let min = hints.min;
    let max = hints.max;

    if min.is_some() && max.is_some() && min.unwrap().0 == max.unwrap().0 && min.unwrap().1 == max.unwrap().1 {
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
      let mut w : Window = uninitialized();

      if XGetTransientForHint(self.display, window, &mut w) != 0 {
        Some(w)
      } else {
        None
      }
    }
  }

  pub fn get_size_hints(&self, window: Window) -> SizeHint {
    unsafe {
      let mut size_hint : XSizeHints = uninitialized();
      let mut tmp : c_long = 0;
      XGetWMNormalHints(self.display, window, &mut size_hint, &mut tmp);

      let min = if size_hint.flags & PMinSize == PMinSize {
          Some((size_hint.min_width as u32, size_hint.min_height as u32))
      } else {
          None
      };

      let max = if size_hint.flags & PMaxSize == PMaxSize {
          Some((size_hint.max_width as u32, size_hint.max_height as u32))
      } else {
          None
      };
      SizeHint { min: min, max: max }
    }
  }

  fn get_wm_hints(&self, window: Window) -> &XWMHints {
    unsafe {
      &*XGetWMHints(self.display, window)
    }
  }

  pub fn is_urgent(&self, window: Window) -> bool {
    let hints = self.get_wm_hints(window);
    hints.flags.contains(Urgency)
  }

  pub fn get_class_name(&self, window: Window) -> String {
    unsafe {
      let mut hint : XClassHint = uninitialized();

      if XGetClassHint(self.display, window, &mut hint) == 0 || hint.res_class.is_null() {
        String::from_str("")
      } else {
        String::from_str(from_c_str(hint.res_class as *const c_char))
      }
    }
  }

  pub fn get_window_title(&self, window: Window) -> String {
    if window == self.root {
      return String::from_str("");
    }

    unsafe {
      let mut name : *mut c_char = uninitialized();
      if XFetchName(self.display, window, &mut name) == 0 || name.is_null() {
        String::from_str("")
      } else {
        String::from_str(from_c_str(name as *const c_char))
      }
    }
  }

  fn cast_event_to<T>(&self) -> &T {
    unsafe {
      &*(self.event as *const T)
    }
  }

  pub fn get_event(&self) -> XlibEvent {
    unsafe {
      XNextEvent(self.display, self.event);
    }

    let evt_type : c_int = *self.cast_event_to();
    match evt_type{
      MapRequest => {
        let evt : &XMapRequestEvent = self.cast_event_to();

        unsafe {
          let atom = self.get_atom("WM_STATE");
          self.change_property(evt.window as u64, atom, atom, 0, &mut [1, 0]);
          self.grab_button(evt.window);
          XSelectInput(self.display, evt.window, 0x420010);
        }

        XMapRequest(evt.window)
      },
      ConfigurationNotify => {
        let evt : &XConfigureEvent = self.cast_event_to();
        if evt.window == self.root {
          XConfigurationNotify(evt.window)
        } else {
          Ignored
        }
      },
      ConfigurationRequest => {
        let event : &XConfigureRequestEvent = self.cast_event_to();
        let changes = WindowChanges{
          x: event.x as u32,
          y: event.y as u32,
          width: event.width as u32,
          height: event.height as u32,
          border_width: event.border_width as u32,
          sibling: event.above as Window,
          stack_mode: event.detail as u32
        };
        XConfigurationRequest(event.window, changes, event.value_mask as u32)
      },
      Destroy => {
        let evt : &XDestroyWindowEvent = self.cast_event_to();
        XDestroy(evt.window)
      },
      UnmapNotify => {
        let evt : &XUnmapEvent = self.cast_event_to();
        XUnmapNotify(evt.window, evt.send_event > 0)
      },
      PropertyNotify => {
        let evt : &XPropertyEvent = self.cast_event_to();
        XPropertyNotify(evt.window, evt.atom, evt.state == 0)
      },
      EnterNotify => {
        let evt : &XEnterWindowEvent = self.cast_event_to();
        if evt.detail != 2 {
          XEnterNotify(evt.window)
        } else {
          Ignored
        }
      },
      FocusOut => {
        let evt : &XFocusOutEvent = self.cast_event_to();
        if evt.detail != 5 {
          XFocusOut(evt.window)
        } else {
          Ignored
        }
      },
      ButtonPress => {
        let evt : &XButtonPressedEvent = self.cast_event_to();
        unsafe {
          XAllowEvents(self.display, 2, 0);
        }

        XButtonPress(evt.window)
      },
      KeyPress => {
        let evt : &XKeyPressedEvent = self.cast_event_to();
        XKeyPress(evt.window, evt.state as u8, self.keycode_to_string(evt.keycode))
      },
      _ => {
        Ignored
      }
    }
  }
}
