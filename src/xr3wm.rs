#![feature(phase)]
#[phase(plugin, link)]
extern crate log;
extern crate xlib;

use std::io::Command;
use xlib::Window;
use xlib_window_system::{XlibWindowSystem, XMapRequest, XEnterNotify, XLeaveNotify, XKeyPress};

mod xlib_window_system;

struct Workspace {
  vroot: Window,
  tag: String,
  screen: uint,
  windows: Vec<Window>
}

struct Workspaces {
  vec: Vec<Workspace>,
  cur: uint
}

impl Workspaces {
  pub fn new(ws: &XlibWindowSystem, count: uint, tags: Vec<String>) -> Workspaces {
    Workspaces{
      vec: Vec::from_fn(9, |idx| {
        Workspace {
          vroot: ws.new_vroot(),
          tag: tags[idx].clone(),
          screen: 0,
          windows: Vec::new()
        }
      }),
      cur: 99
    }
  }

  pub fn get_current(&mut self) -> &mut Workspace {
    self.vec.get_mut(self.cur)
  }

  pub fn change_to(&mut self, ws: &XlibWindowSystem, index: uint) {
     if self.cur != index {
      println!("Workspace {}", index + 1);
      self.cur = index;
      ws.raise_window(self.vec[index].vroot);
    }
  }
}

fn main() {
  let ws = &XlibWindowSystem::new().unwrap();

  let mut workspaces = Workspaces::new(ws, 9, Vec::from_fn(9, |idx| idx.to_string()));
  workspaces.change_to(ws, 0);

  loop {
    match ws.get_event() {
      XMapRequest(window) => {
        println!("MapRequest");
        ws.setup_window(0, 0, ws.get_display_width(0)/2, ws.get_display_height(0), window);
        ws.map_to_parent(workspaces.get_current().vroot, window);
        workspaces.get_current().windows.push(window);
      },
      XEnterNotify(window, detail) => {
        if detail != 2 {
          ws.set_window_border_color(window, 0x0000FF00);
        }
      },
      XLeaveNotify(window, detail) => {
        if detail != 2 {
          ws.set_window_border_color(window, 0x00FF0000);
        }
      },
      XKeyPress(window, state, keycode) => {
        if state == 80 {
          if keycode > 9 && keycode < 19 {
            workspaces.change_to(ws, keycode - 10);
          } else if keycode == 36 {
            spawn(proc() { Command::new("xterm").spawn(); });
          }
        }
      },
      Unknown => {

      }
    }
  }
}
