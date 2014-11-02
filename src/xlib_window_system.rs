extern crate xlib;
extern crate libc;

use std::ptr::null_mut;
use self::libc::{c_void};
use self::libc::funcs::c95::stdlib::malloc;
use self::xlib::{Display,
                 Window,
                 XOpenDisplay,
                 XDefaultRootWindow,
                 XSelectInput,
                 XDisplayWidth,
                 XDisplayHeight};

const KeyPress         : uint = 2;
const KeyRelease       : uint = 3;
const ButtonPress      : uint = 4;
const ButtonRelease    : uint = 5;
const MotionNotify     : uint = 6;
const EnterNotify      : uint = 7;
const LeaveNotify      : uint = 8;
const FocusIn          : uint = 9;
const FocusOut         : uint = 10;
const KeymapNotify     : uint = 11;
const Expose           : uint = 12;
const GraphicsExpose   : uint = 13;
const NoExpose         : uint = 14;
const VisibilityNotify : uint = 15;
const CreateNotify     : uint = 16;
const DestroyNotify    : uint = 17;
const UnmapNotify      : uint = 18;
const MapNotify        : uint = 19;
const MapRequest       : uint = 20;
const ReparentNotify   : uint = 21;
const ConfigureNotify  : uint = 22;
const ConfigureRequest : uint = 23;
const GravityNotify    : uint = 24;
const ResizeRequest    : uint = 25;
const CirculateNotify  : uint = 26;
const CirculateRequest : uint = 27;
const PropertyNotify   : uint = 28;
const SelectionClear   : uint = 29;
const SelectionRequest : uint = 30;
const SelectionNotify  : uint = 31;
const ColormapNotify   : uint = 32;
const ClientMessage    : uint = 33;
const MappingNotify    : uint = 34;

pub struct XlibWindowSystem {
  display: *mut Display,
  root: Window,
  event: *mut c_void
}

impl XlibWindowSystem {
  pub fn new() -> Option<XlibWindowSystem> {
    unsafe {
      let display = XOpenDisplay(null_mut());
      if display.is_null() {
        return None;
      }

      let root = XDefaultRootWindow(display);

      XSelectInput(display, root, 0x180030);

      Some(XlibWindowSystem{
        display: display,
        root: root,
        event: malloc(256)
      })
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
}