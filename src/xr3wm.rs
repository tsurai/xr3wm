extern crate xlib;

use std::io::Command;
use layout::{Layout, TallLayout};
use xlib::Window;
use xlib_window_system::{ XlibWindowSystem,
                          XMapRequest,
                          XConfigurationRequest,
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
  windows: Vec<Window>,
  layout: Box<Layout + 'static>
}

impl Workspace {
  pub fn add_window(&mut self, ws: &XlibWindowSystem, window: Window) {
    self.windows.push(window);
    self.apply_layout(ws);
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, index: uint) {
    self.windows.remove(index);
    self.apply_layout(ws);
  }

  fn apply_layout(&self, ws: &XlibWindowSystem) {
    for (i,rect) in self.layout.apply(ws.get_display_rect(0), &self.windows).iter().enumerate() {
      ws.setup_window(rect.x as uint, rect.y as uint, rect.width as uint, rect.height as uint, self.vroot, self.windows[i]);
    }
  }

  pub fn index_of(&self, window: Window) -> Option<uint> {
    self.windows.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last()
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
          screen: count,
          windows: Vec::new(),
          layout: TallLayout::new(1),
        }
      }),
      cur: 99,
    }
  }

  pub fn get_current(&mut self) -> &mut Workspace {
    self.vec.get_mut(self.cur)
  }

  pub fn get_by_vroot(&mut self, vroot: Window) -> &mut Workspace {
    let index = self.vec.iter().enumerate().find(|&(_,ref w)| w.vroot == vroot).unwrap().val0();
    self.vec.get_mut(index)
  }

  pub fn change_to(&mut self, ws: &XlibWindowSystem, index: uint) {
     if self.cur != index {
      self.cur = index;
      ws.raise_window(self.vec[index].vroot);
    }
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, window: Window) {
    for workspace in self.vec.iter_mut() {
      match workspace.index_of(window) {
        Some(index) => {
          workspace.remove_window(ws, index);
          return;
        },
        None => {}
      }
    }
  }
}

fn main() {
  let mut ws = &mut XlibWindowSystem::new().unwrap();

  let mut workspaces = Workspaces::new(ws, 9, Vec::from_fn(9, |idx| idx.to_string()));
  workspaces.change_to(ws, 0);

  loop {
    match ws.get_event() {
      XMapRequest(window) => {
        workspaces.get_current().add_window(ws, window);
      },
      XDestroyNotify(window) => {
        workspaces.remove_window(ws, window);
      },
      XConfigurationRequest(window, changes, mask) => {
        ws.configure_window(window, changes, mask);
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
            spawn(proc() { Command::new("xterm").arg("-class").arg("UXTerm").arg("-u8").spawn(); });
          }
        }
      },
      Unknown => {

      }
    }
  }
}
