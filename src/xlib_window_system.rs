extern crate libc;

use std::ptr::null_mut;
use std::mem::{uninitialized, transmute};
use std::str::raw::c_str_to_static_slice;
use self::libc::{c_void, c_int, c_char};
use self::libc::funcs::c95::stdlib::malloc;
use xlib::{ Display,
            Window,
            XOpenDisplay,
            XDefaultRootWindow,
            XSelectInput,
            XDisplayWidth,
            XDisplayHeight,
            XSync,
            XNextEvent,
            XMapWindow,
            XReparentWindow,
            XMoveWindow,
            XResizeWindow,
            XMoveResizeWindow,
            XSetWindowBorderWidth,
            XSetWindowBorder,
            XFetchName,
            XCreateSimpleWindow,
            XMapRequestEvent,
            XDestroyWindowEvent,
            XEnterWindowEvent,
            XKeyPressedEvent,
            XLeaveWindowEvent,
            XRaiseWindow
          };

const KeyPress               : uint = 2;
const KeyRelease             : uint = 3;
const ButtonPress            : uint = 4;
const ButtonRelease          : uint = 5;
const MotionNotify           : uint = 6;
const EnterNotify            : uint = 7;
const LeaveNotify            : uint = 8;
const FocusIn                : uint = 9;
const FocusOut               : uint = 10;
const KeymapNotify           : uint = 11;
const Expose                 : uint = 12;
const GraphicsExpose         : uint = 13;
const NoExpose               : uint = 14;
const VisibilityNotify       : uint = 15;
const CreateNotify           : uint = 16;
const DestroyNotify          : uint = 17;
const UnmapNotify            : uint = 18;
const MapNotify              : uint = 19;
const MapRequest             : uint = 20;
const ReparentNotify         : uint = 21;
const ConfigureNotify        : uint = 22;
const ConfigureRequest       : uint = 23;
const GravityNotify          : uint = 24;
const ResizeRequest          : uint = 25;
const CirculateNotify        : uint = 26;
const CirculateRequest       : uint = 27;
const PropertyNotify         : uint = 28;
const SelectionClear         : uint = 29;
const SelectionRequest       : uint = 30;
const SelectionNotify        : uint = 31;
const ColormapNotify         : uint = 32;
const ClientMessage          : uint = 33;
const MappingNotify          : uint = 34;

const NotifyAncestor         : uint = 0;
const NotifyVirtual          : uint = 1;
const NotifyInferior         : uint = 2;
const NotifyNonlinear        : uint = 3;
const NotifyNonlinearVirtual : uint = 4;
const NotifyPointer          : uint = 5;
const NotifyPointerRoot      : uint = 6;
const NotifyDetailNone       : uint = 7;

pub struct XlibWindowSystem {
  display:        *mut Display,
  root:           Window,
  event:          *mut c_void
}

pub enum XlibEvent {
  XMapRequest(Window),
  XUnmapNotify(Window),
  XDestroyNotify(Window),
  XEnterNotify(Window, uint),
  XLeaveNotify(Window, uint),
  XKeyPress(Window, uint, uint),
  Unknown
}

impl XlibWindowSystem {
  pub fn new() -> Option<XlibWindowSystem> {
    unsafe {
      let display = XOpenDisplay(null_mut());
      if display.is_null() {
        return None;
      }

      let root = XDefaultRootWindow(display);
      XSelectInput(display, root, 0x100031);

      Some(XlibWindowSystem{
        display: display,
        root: root,
        event: malloc(256)
      })
    }
  }

  pub fn new_vroot(&self) -> Window {
    unsafe {
      let window = XCreateSimpleWindow(self.display, self.root, 0, 0, self.get_display_width(0), self.get_display_height(0), 0, 0, 0);
      XMapWindow(self.display, window);
      window
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

  pub fn map_to_parent(&self, parent: Window, window: Window) {
    unsafe {
      XReparentWindow(self.display, window, parent, 0, 0);
      XMapWindow(self.display, window);
    }
  }

  pub fn move_window(&self, window: Window, x: uint, y: uint) {
    unsafe {
      XMoveWindow(self.display, window, x as i32, y as i32);
    }
  }

  pub fn resize_window(&self, window: Window, width: uint, height: uint) {
    unsafe {
      // TODO: the borderwidth should not be hardcoded
      XResizeWindow(self.display, window, width as u32 - 2, height as u32 - 2);
    }
  }

  pub fn move_resize_window(&self, window: Window, x: uint, y: uint, width: uint, height: uint) {
    unsafe {
      XMoveResizeWindow(self.display, window, x as i32, y as i32, width as u32, height as u32);
    }
  }

  pub fn raise_window(&self, window: Window) {
    unsafe {
      XRaiseWindow(self.display, window);
    }
  }

  pub fn set_window_border_width(&self, window: Window, width: uint) {
    if window != self.root {
      unsafe {
        XSetWindowBorderWidth(self.display, window, width as u32);
      }
    }
  }

  pub fn set_window_border_color(&self, window: Window, color: u64) {
    if window != self.root {
      unsafe {
        XSetWindowBorder(self.display, window, color);
      }
    }
  }

  pub fn setup_window(&self, x: uint, y: uint, width: uint, height: uint, vroot: Window, window: Window) {
    unsafe {
      XSelectInput(self.display, window, 0x020031);
    }

    self.set_window_border_width(window, 1);
    self.set_window_border_color(window, 0x00FF0000);

    self.map_to_parent(vroot, window);
    self.move_resize_window(window, x, y, width, height);
  }

  fn get_window_name(&self, window: Window) -> String {
    if window == self.root {
      return String::from_str("root");
    }

    unsafe {
      let mut name : *mut c_char = uninitialized();
      XFetchName(self.display, window, &mut name);
      String::from_str(c_str_to_static_slice(transmute(name)))
    }
  }

  fn cast_event_to<T>(&self) -> &T {
    unsafe {
      let evt_ptr : *const T = transmute(self.event);
      let ref evt = *evt_ptr;
      evt
    }
  }

  pub fn get_event(&self) -> XlibEvent {
    unsafe {
      XNextEvent(self.display, self.event);
    }

    let evt_type : c_int = *self.cast_event_to();
    match evt_type as uint {
      MapRequest => {
        let evt : &XMapRequestEvent = self.cast_event_to();
        XMapRequest(evt.window)
      },
      DestroyNotify => {
        let evt : &XDestroyWindowEvent = self.cast_event_to();
        XDestroyNotify(evt.window)
      },
      EnterNotify => {
        let evt: &XEnterWindowEvent = self.cast_event_to();
        XEnterNotify(evt.window, evt.detail as uint)
      },
      LeaveNotify => {
        let evt: &XLeaveWindowEvent = self.cast_event_to();
        XLeaveNotify(evt.window, evt.detail as uint)
      },
      KeyPress => {
        let evt: &XKeyPressedEvent = self.cast_event_to();
        XKeyPress(evt.window, evt.state as uint, evt.keycode as uint)
      }
      _ => {
        Unknown
      }
    }
  }
}