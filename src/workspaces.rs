use config::Config;
use layout::{Layout, LayoutBox};
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use self::MoveOp::*;

pub struct WorkspaceConfig {
  pub tag: String,
  pub screen: u32,
  pub layout: LayoutBox
}

pub struct Workspace {
  windows: Vec<Window>,
  focused_window: Window,
  tag: String,
  screen: u32,
  visible: bool,
  layout: Box<Layout + 'static>
}

pub enum MoveOp {
  Up,
  Down,
  Swap
}

impl Workspace {
  pub fn add_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    self.windows.push(window);

    if self.visible {
      ws.map_window(window);
      self.redraw(ws, config);
    }
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    ws.unmap_window(self.windows[index]);
    self.focused_window = 0;
    self.windows.remove(index);

    let new_focused_window = if !self.windows.is_empty() {
      if index < self.windows.len() {
        self.windows[index]
      } else {
        self.windows[index - 1]
      }
    } else {
      0
    };

    if self.visible {
      self.redraw(ws, config);
      self.focus_window(ws, config, new_focused_window);
    }
  }

  pub fn focus_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    if window == 0 {
      return;
    }

    self.unfocus_window(ws, config);

    self.focused_window = window;
    ws.focus_window(window, config.border_focus_color);
  }

  pub fn unfocus_window(&mut self, ws: &XlibWindowSystem, config: &Config) {
    if self.focused_window != 0 {
      ws.set_window_border_color(self.focused_window, config.border_color);
      self.focused_window = 0;
    }
  }

  pub fn move_focus(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
    if self.focused_window == 0 || self.windows.len() < 2 {
      return;
    }

    let index = self.index_of(self.focused_window).unwrap();
    let new_focused_window = match op {
      Up => {
        if index == 0 {
          self.windows[self.windows.len() - 1]
        } else {
          self.windows[index - 1]
        }
      },
      Down => {
        self.windows[(index + 1) % self.windows.len()]
      },
      Swap => {
        self.windows[0]
      }
    };

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

    self.redraw(ws, config);
  }

  pub fn index_of(&self, window: Window) -> Option<uint> {
    self.windows.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last()
  }

  pub fn get_focused_window(&self) -> Window {
    self.focused_window
  }

  pub fn hide(&mut self, ws: &XlibWindowSystem) {
    self.visible = false;

    for &w in self.windows.iter() {
      ws.unmap_window(w);
    }
  }

  pub fn show(&mut self, ws: &XlibWindowSystem, config: &Config) {
    self.visible = true;

    self.redraw(ws, config);
    for &w in self.windows.iter() {
      ws.map_window(w);
    }
  }

  pub fn redraw(&self, ws: &XlibWindowSystem, config: &Config) {
    for (i,rect) in self.layout.apply(ws.get_display_rect(0), &self.windows).iter().enumerate() {
      ws.setup_window(rect.x, rect.y, rect.width, rect.height, config.border_width, config.border_color, self.windows[i]);
    }

    ws.sync();
    ws.set_window_border_color(self.focused_window, config.border_focus_color);
  }
}

pub struct Workspaces {
  list: Vec<Workspace>,
  cur: uint
}

impl Workspaces {
  pub fn new(config: &mut Config) -> Workspaces{
    Workspaces{
      list: config.workspaces.iter_mut().map(|c| {
        Workspace {
          windows: Vec::new(),
          focused_window: 0,
          tag: c.tag.clone(),
          screen: c.screen,
          visible: false,
          layout: (c.layout)()
        }
      }).collect(),
      cur: 99,
    }
  }

  pub fn get_current(&mut self) -> &mut Workspace {
    self.list.get_mut(self.cur).unwrap()
  }

  pub fn change_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    if self.cur != index && index < self.list.len() {
      if self.cur != 99 {
        self.list[self.cur].hide(ws);
      }

      self.cur = index;
      let focused_window = self.list[index].get_focused_window();

      self.list[index].show(ws, config);
      self.list[index].focus_window(ws, config, focused_window);
    }
  }

  pub fn move_window_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    let window = self.list[self.cur].get_focused_window();
    if window == 0 {
      return;
    }

    self.remove_window(ws, config, window);
    self.list[index].add_window(ws, config, window);
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    for workspace in self.list.iter_mut() {
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