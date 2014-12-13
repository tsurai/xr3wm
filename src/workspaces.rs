use config::Config;
use layout::{Layout, LayoutBox};
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use self::MoveOp::*;

pub struct WorkspaceConfig {
  pub tag: String,
  pub screen: uint,
  pub layout: LayoutBox
}

pub struct Workspace {
  managed: Vec<Window>,
  unmanaged: Vec<Window>,
  focused_window: Window,
  tag: String,
  screen: uint,
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
    if !ws.is_transient_for(window) {
      self.managed.push(window);
    } else {
      self.unmanaged.push(window);
    }

    if self.visible {
      ws.map_window(window);
      self.redraw(ws, config);
    }
  }

  fn is_managed(&self, window: Window) -> bool {
    self.managed.iter().find(|&x| *x == window).is_some()
  }

  fn is_unmanaged(&self, window: Window) -> bool {
    self.unmanaged.iter().find(|&x| *x == window).is_some()
  }

  fn get_managed(&self, window: Window) -> uint {
    self.managed.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last().unwrap()
  }

  fn get_unmanaged(&self, window: Window) -> uint {
    self.unmanaged.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last().unwrap()
  }

  fn remove_managed(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    let index = self.get_managed(window);

    self.focused_window = 0;
    ws.unmap_window(window);
    self.managed.remove(index);

    let new_focused_window = if !self.managed.is_empty() {
      self.managed[if index < self.managed.len() { index } else { index - 1}]
    } else {
      0
    };

    if self.visible {
      self.redraw(ws, config);
      self.focus_window(ws, config, new_focused_window);
    }
  }

  fn remove_unmanaged(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    let index = self.get_unmanaged(window);

    self.focused_window = 0;
    ws.unmap_window(window);
    self.unmanaged.remove(index);

    let new_focused_window = if !self.unmanaged.is_empty() {
      self.unmanaged[if index < self.unmanaged.len() { index } else { index - 1}]
    } else {
      0
    };

    if self.visible {
      self.redraw(ws, config);
      self.focus_window(ws, config, new_focused_window);
    }
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) -> bool {
    if self.is_managed(window) {
      self.remove_managed(ws, config, window);
    } else {
      if self.is_unmanaged(window) {
        self.remove_unmanaged(ws, config, window);
      } else {
        return false;
      }
    }

    return true;
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
    if self.focused_window == 0 || self.managed.len() < 2 {
      return;
    }

    let index = self.index_of(self.focused_window).unwrap();
    let new_focused_window = match op {
      Up => {
        if index == 0 {
          self.managed[self.managed.len() - 1]
        } else {
          self.managed[index - 1]
        }
      },
      Down => {
        self.managed[(index + 1) % self.managed.len()]
      },
      Swap => {
        self.managed[0]
      }
    };

    self.focus_window(ws, config, new_focused_window);
  }

  pub fn move_window(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
    if self.focused_window == 0 || self.managed.len() < 2 {
      return;
    }

    let pos = self.index_of(self.focused_window).unwrap();
    let new_pos = match op {
      Up => {
        if pos == 0 {
          self.managed.len() - 1
        } else {
          pos - 1
        }
      },
      Down => {
        (pos + 1) % self.managed.len()
      },
      Swap => {
        let master = self.managed[0];
        self.managed.insert(pos, master);
        self.managed.remove(0);
        0
      }
    };

    self.managed.remove(pos);
    self.managed.insert(new_pos, self.focused_window);

    self.redraw(ws, config);
  }

  pub fn index_of(&self, window: Window) -> Option<uint> {
    self.managed.iter().enumerate().filter(|&(_,&w)| w == window).map(|(i,_)| i).last()
  }

  pub fn get_focused_window(&self) -> Window {
    self.focused_window
  }

  pub fn unfocus(&mut self, ws: &XlibWindowSystem, config: &Config) {
    ws.set_window_border_color(self.focused_window, config.border_color);
  }

  pub fn focus(&self, ws: &XlibWindowSystem, config: &Config) {
    if self.focused_window != 0 {
      ws.focus_window(self.focused_window, config.border_focus_color);
    }
  }

  pub fn hide(&mut self, ws: &XlibWindowSystem) {
    self.visible = false;

    for &w in self.managed.iter() {
      ws.unmap_window(w);
    }

    for &w in self.unmanaged.iter() {
      ws.unmap_window(w);
    }
  }

  pub fn show(&mut self, ws: &XlibWindowSystem, config: &Config) {
    self.visible = true;

    self.redraw(ws, config);
    for &w in self.managed.iter() {
      ws.map_window(w);
    }

    for &w in self.unmanaged.iter() {
      ws.map_window(w);
    }
  }

  pub fn redraw(&self, ws: &XlibWindowSystem, config: &Config) {
    for (i,rect) in self.layout.apply(ws.get_screen_infos()[self.screen], &self.managed).iter().enumerate() {
      ws.setup_window(rect.x, rect.y, rect.width, rect.height, config.border_width, config.border_color, self.managed[i]);
    }
  }
}

pub struct Workspaces {
  list: Vec<Workspace>,
  cur: uint
}

impl Workspaces {
  pub fn new(config: &mut Config, screens: uint) -> Workspaces{
    let mut workspaces = Workspaces{
      list: config.workspaces.iter_mut().map(|c| {
        Workspace {
          managed: Vec::new(),
          unmanaged: Vec::new(),
          focused_window: 0,
          tag: c.tag.clone(),
          screen: c.screen,
          visible: false,
          layout: (c.layout)()
        }
      }).collect(),
      cur: 0,
    };

    for screen in range(0u, screens) {
      if workspaces.list.iter().find(|ws| ws.screen == screen).is_none() {
        match workspaces.list.iter_mut().filter(|ws| ws.screen == 0).nth(1) {
          Some(ws) => {
            ws.screen = screen;
          },
          None => {}
        }
      }
    }

    for screen in range(0u, screens) {
      let ws = workspaces.list.iter_mut().find(|ws| ws.screen == screen).unwrap();
      ws.visible = true;
    }

    workspaces
  }

  pub fn current(&mut self) -> &mut Workspace {
    self.list.get_mut(self.cur).unwrap()
  }

  pub fn switch_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    if self.cur != index && index < self.list.len() {
      self.list[self.cur].hide(ws);

      if self.list[index].visible {
        self.switch_screens(index);
        self.list[self.cur].show(ws, config);
      }

      self.list[index].show(ws, config);
      self.list[index].focus(ws, config);
      self.cur = index;
    }
  }

  pub fn switch_to_screen(&mut self, ws: &XlibWindowSystem, config: &Config, screen: uint) {
    match self.list.iter().enumerate().filter(|&(_,ws)| ws.screen == screen && ws.visible).map(|(i,_)| i).last() {
      Some(index) => {
        self.list[self.cur].unfocus(ws, config);
        self.list[index].focus(ws, config);
        self.cur = index;
      },
      None => { }
    }
  }

  pub fn move_window_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    let window = self.list[self.cur].get_focused_window();
    if window == 0 {
      return;
    }

    ws.unmap_window(window);
    self.remove_window(ws, config, window);
    self.list[index].add_window(ws, config, window);
  }

  pub fn move_window_to_screen(&mut self, ws: &XlibWindowSystem, config: &Config, screen: uint) {
    match self.list.iter().enumerate().find(|&(_,ws)| ws.screen == screen) {
      Some((index,_)) => {
        self.move_window_to(ws, config, index);
      },
      None => {}
    }
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    for workspace in self.list.iter_mut() {
      if workspace.remove_window(ws, config, window) {
        return;
      }
    }
  }

  fn switch_screens(&mut self, dest: uint) {
    let screen = self.list[self.cur].screen;
    self.list[self.cur].screen = self.list[dest].screen;
    self.list[dest].screen = screen;
  }
}