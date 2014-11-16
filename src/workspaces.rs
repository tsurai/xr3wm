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
  focused_window: Window,
  tag: String,
  screen: u32,
  layout: Rc<Box<Layout + 'static>>
}

pub enum MoveOp {
  Up,
  Down,
  Swap
}

impl Workspace {
  pub fn add_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    self.windows.push(window);
    ws.map_to_parent(self.vroot, window);
    self.apply_layout(ws, config);
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    self.windows.remove(index);
    self.apply_layout(ws, config);

    if !self.windows.is_empty() {
      let new_focused_window : Window = if index < self.windows.len() {
        self.windows[index]
      } else {
        self.windows[index - 1]
      };

      self.focus_window(ws, config, new_focused_window);
    }
  }

  pub fn focus_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    if window == 0 {
      return;
    }

    if self.focused_window != 0 {
      ws.set_window_border_color(self.focused_window, config.border_color);
    }

    self.focused_window = window;
    ws.focus_window(window, config.border_focus_color);
    ws.sync();
  }

  pub fn unfocus_window(&mut self, ws: &XlibWindowSystem, config: &Config) {
    ws.set_window_border_color(self.focused_window, config.border_color);
    self.focused_window = 0;
  }

  pub fn move_focus_up(&mut self, ws: &XlibWindowSystem, config: &Config) {
    if self.focused_window == 0 || self.windows.len() < 2 {
      return;
    }

    let index = self.index_of(self.focused_window).unwrap();
    let new_focused_window = if index == 0 {
      self.windows[self.windows.len() - 1]
    } else {
      self.windows[index - 1]
    };

    self.focus_window(ws, config, new_focused_window);
  }

  pub fn move_focus_down(&mut self, ws: &XlibWindowSystem, config: &Config) {
    if self.focused_window == 0 || self.windows.len() < 2 {
      return;
    }

    let index = self.index_of(self.focused_window).unwrap();
    let new_focused_window = self.windows[(index + 1) % self.windows.len()];

    self.focus_window(ws, config, new_focused_window);
  }

  pub fn move_window(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
    if self.focused_window == 0 || self.windows.len() < 2 {
      return;
    }

    let pos = self.index_of(self.focused_window).unwrap();
    let new_pos = match op {
      Up => {
        if pos == 0 {
          self.windows.len() - 1
        } else {
          pos - 1
        }
      },
      Down => {
        (pos + 1) % self.windows.len()
      },
      Swap => {
        let master = self.windows[0];
        self.windows.insert(pos, master);
        self.windows.remove(0);
        0
      }
    };

    self.windows.remove(pos);
    self.windows.insert(new_pos, self.focused_window);

    self.apply_layout(ws, config);
  }

  pub fn index_of(&self, window: Window) -> Option<uint> {
    self.windows.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last()
  }

  pub fn get_focused_window(&self) -> Window {
    self.focused_window
  }

  fn apply_layout(&self, ws: &XlibWindowSystem, config: &Config) {
    for (i,rect) in self.layout.apply(ws.get_display_rect(0), &self.windows).iter().enumerate() {
      ws.setup_window(rect.x, rect.y, rect.width, rect.height, config.border_width, config.border_color, self.windows[i]);
    }

    ws.sync();
    ws.set_window_border_color(self.focused_window, config.border_focus_color);
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
          focused_window: 0,
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

  pub fn change_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
     if self.cur != index && index < self.vec.len() {
      let focused_window = self.vec[index].get_focused_window();
      self.cur = index;

      ws.raise_window(self.vec[index].vroot);
      self.get_current().focus_window(ws, config, focused_window);
    }
  }

  pub fn move_window_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    let window = self.get_current().get_focused_window();
    if window == 0 {
      return;
    }

    self.remove_window(ws, config, window);
    self.vec[index].add_window(ws, config, window);
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