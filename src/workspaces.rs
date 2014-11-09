use config::Config;
use layout::Layout;
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use std::rc::Rc;

pub struct WorkspaceConfig {
  pub tag: String,
  pub screen: u32,
  pub layout: Rc<Box<Layout + 'static>>
}

pub struct Workspace {
  vroot: Window,
  windows: Vec<Window>,
  tag: String,
  screen: u32,
  layout: Rc<Box<Layout + 'static>>
}

impl Workspace {
  pub fn add_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    self.windows.push(window);
    self.apply_layout(ws, config);
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    self.windows.remove(index);
    self.apply_layout(ws, config);
  }

  fn apply_layout(&self, ws: &XlibWindowSystem, config: &Config) {
    for (i,rect) in self.layout.apply(ws.get_display_rect(0), &self.windows).iter().enumerate() {
      ws.setup_window(rect.x, rect.y, rect.width, rect.height, config.border_width, config.border_color, self.vroot, self.windows[i]);
    }
  }

  pub fn index_of(&self, window: Window) -> Option<uint> {
    self.windows.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last()
  }
}

pub struct Workspaces {
  vec: Vec<Workspace>,
  cur: uint
}

impl Workspaces {
  pub fn new(ws: &XlibWindowSystem, config: &Config) -> Workspaces{
    Workspaces{
      vec: config.workspaces.iter().map(|c| {
        Workspace {
          vroot: ws.new_vroot(),
          windows: Vec::new(),
          tag: c.tag.clone(),
          screen: c.screen,
          layout: c.layout.clone()
        }
      }).collect(),
      cur: 99,
    }
  }

  pub fn get_current(&mut self) -> &mut Workspace {
    self.vec.get_mut(self.cur).unwrap()
  }

  pub fn change_to(&mut self, ws: &XlibWindowSystem, index: uint) {
     if self.cur != index && index < self.vec.len() {
      self.cur = index;
      ws.raise_window(self.vec[index].vroot);
    }
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    for workspace in self.vec.iter_mut() {
      match workspace.index_of(window) {
        Some(index) => {
          workspace.remove_window(ws, config, index);
          return;
        },
        None => {}
      }
    }
  }
}