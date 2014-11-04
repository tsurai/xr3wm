extern crate xlib;

use std::io::Command;
use layout::{Layout, TallLayout};
use xlib::Window;
use xlib_window_system::{ XlibWindowSystem,
                          XMapRequest,
                          XDestroyNotify,
                          XEnterNotify,
                          XLeaveNotify,
                          XKeyPress};

mod xlib_window_system;
mod layout;

struct Workspace {
  vroot: Window,
  tag: String,
  screen: uint,
  layout: Box<Layout + 'static>
}

impl Workspace {
  pub fn add_window(&mut self, ws: &XlibWindowSystem, window: Window) {
    self.layout.add_window(ws, self.vroot, window)
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, window: Window) {
    self.layout.remove_window(ws, self.vroot, window)
  }
}

struct Workspaces {
  vec: Vec<Workspace>,
  cur: uint
}

impl Workspaces {
  pub fn new(ws: &XlibWindowSystem, count: uint, tags: Vec<String>) -> Workspaces{
    Workspaces{
      vec: Vec::from_fn(9, |idx| {
        Workspace {
          vroot: ws.new_vroot(),
          tag: tags[idx].clone(),
          screen: 0,
          layout: TallLayout::new(),
        }
      }),
      cur: 99,
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
        workspaces.get_current().add_window(ws, window);
      },
      XDestroyNotify(window) => {
        println!("destroy {}", window);
      },
      XEnterNotify(window, detail) => {
        if detail != 2 {
          println!("enter notify {}", window);
          ws.set_window_border_color(window, 0x0000FF00);
        }
      },
      XLeaveNotify(window, detail) => {
        if detail != 2 {
          println!("leave notify {}", window);
          ws.set_window_border_color(window, 0x00FF0000);
        }
      },
      XKeyPress(window, state, keycode) => {
        if state == 80 {
          if keycode > 9 && keycode < 19 {
            workspaces.change_to(ws, keycode - 10);
          } else if keycode == 36 {
            spawn(proc() { Command::new("xterm").arg("-class").arg("UXTerm").arg("-u8").spawn(); });
          }
        }
      },
      Unknown => {

      }
    }
  }
}
