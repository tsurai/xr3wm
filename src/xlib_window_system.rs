#![allow(non_upper_case_globals)]
extern crate libc;

use keycode::{MOD_2, MOD_LOCK};
use layout::Rect;
use std::str;
use std::fmt;
use std::os::env;
use std::c_vec::CVec;
use std::ptr::null_mut;
use std::mem::{uninitialized, transmute};
use std::slice::from_raw_buf;
use self::libc::{c_void, c_int, c_uint, c_char, c_long, c_ulong};
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
  XEnterNotify(Window),
  XFocusOut(Window),
  XKeyPress(Window, u8, String),
  XButtonPress(Window),
  Ignored
}

pub struct WindowChanges {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
  pub border_width: u32,
  pub sibling: Window,
  pub stack_mode: u32,
}

impl fmt::Show for WindowChanges {
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
      XSelectInput(display, root, 0x1A0030);
      XDefineCursor(display, root, XCreateFontCursor(display, 68));

      XSetErrorHandler(error_handler as *mut u8);

      XlibWindowSystem {
        display: display,
        root: root,
        event: malloc(256)
      }
    }
  }

  pub fn setup_window(&self, x: u32, y: u32, width: u32, height: u32, border_width: u32, border_color: u32, window: Window) {
    self.set_window_border_width(window, border_width);
    self.set_window_border_color(window, border_color);
    self.move_resize_window(window, x, y, width - (2 * border_width), height - (2 * border_width));
  }

  pub fn configure_window(&self, window: Window, window_changes: WindowChanges, mask: u32) {
    unsafe {
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
    }
  }

  pub fn unmap_window(&self, window: Window) {
    unsafe {
      XUnmapWindow(self.display, window);
    }
  }

  pub fn map_window(&self, window: Window) {
    unsafe {
      XMapWindow(self.display, window);
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
      self.sync();
    }
  }

  fn has_protocol(&self, window: Window, protocol: &str) -> bool {
    unsafe {
      let mut count : c_int = uninitialized();
      let mut atoms : *mut Atom = uninitialized();

      XGetWMProtocols(self.display, window, &mut atoms, &mut count);
      CVec::new(atoms, count as uint).as_slice().contains(&XInternAtom(self.display, protocol.to_c_str().as_mut_ptr(), 1))
    }
  }

  pub fn kill_window(&self, window: Window) {
    if window == 0 {
      return;
    }

    unsafe {
      if self.has_protocol(window, "WM_DELETE_WINDOW") {
        let mut msg : XClientMessageEvent = uninitialized();
        msg._type = 33;
        msg.format = 32;
        msg.display = self.display;
        msg.window = window;
        msg.message_type = XInternAtom(self.display, "WM_PROTOCOLS".to_c_str().as_mut_ptr(), 1);
        msg.set_l(&[XInternAtom(self.display, "WM_DELETE_WINDOW".to_c_str().as_mut_ptr(), 1) as u64, 0, 0, 0, 0]);

        XSendEvent(self.display, window, 0, 0, transmute(&msg));
      } else {
        XKillClient(self.display, window);
      }
    }
  }

  pub fn restack_windows(&self, mut windows: Vec<Window>) {
    unsafe {
      XRestackWindows(self.display, windows.as_mut_slice().as_mut_ptr(), windows.len() as i32);
    }
  }

  pub fn sync(&self) {
    unsafe {
      XSync(self.display, 1);
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

      from_raw_buf(&screen_ptr, num as uint).iter().map(|ref screen_info|
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
    if self.is_transient_for(window) {
      return true;
    }

    if let Some(hints) = self.get_size_hints(window) {
      return hints.min_width == hints.max_width && hints.min_height == hints.max_height;
    }

    return false;
  }

  fn is_transient_for(&self, window: Window) -> bool {
    unsafe {
      let mut w : Window = uninitialized();

      return XGetTransientForHint(self.display, window, &mut w) == 1;
    }
  }

  pub fn get_size_hints(&self, window: Window) -> Option<XSizeHints> {
    unsafe {
      let mut ret : c_long = uninitialized();
      let mut hints : XSizeHints = uninitialized();

      if XGetWMNormalHints(self.display, window, &mut hints, &mut ret) != 0 {
        Some(hints)
      } else {
        None
      }
    }
  }

  pub fn get_class_name(&self, window: Window) -> String {
    unsafe {
      let mut hint : XClassHint = uninitialized();
      XGetClassHint(self.display, window, &mut hint);
      String::from_str(str::from_c_str(transmute(hint.res_class)))
    }
  }

  pub fn get_window_title(&self, window: Window) -> String {
    if window == self.root {
      return String::from_str("");
    }

    unsafe {
      let mut name : *mut c_char = uninitialized();
      if XFetchName(self.display, window, &mut name) == 3 || name.is_null() {
        String::from_str("")
      } else {
        String::from_str(str::from_c_str(transmute(name)))
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
          XSelectInput(self.display, evt.window, 0x420030);
          self.grab_button(evt.window);
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
        debug!("UnmapNotify {}", evt.send_event);
        XUnmapNotify(evt.window, evt.send_event > 0)
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
