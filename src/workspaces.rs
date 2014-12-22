use config::Config;
use layout::{Layout, LayoutBox};
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use self::MoveOp::*;
use std::cmp;

struct Stack {
  hidden: Vec<Window>,
  visible: Vec<Window>,
  focused_window: Window
}

impl Stack {
  fn new() -> Stack {
    Stack {
      hidden: Vec::new(),
      visible: Vec::new(),
      focused_window: 0
    }
  }

  fn all(&self) -> Vec<Window> {
    self.hidden.iter().chain(self.visible.iter()).map(|&x| x).collect()
  }

  fn all_mut(&mut self) -> Vec<Window> {
    self.hidden.iter_mut().chain(self.visible.iter_mut()).map(|&x| x).collect()
  }

  fn len(&self) -> uint {
    self.hidden.len() + self.visible.len()
  }

  fn contains(&self, window: Window) -> bool {
    self.all().iter().any(|&x| x == window)
  }

  fn index_of(&self, window: Window) -> uint {
    self.all().iter().enumerate().find(|&(_,&w)| w == window).map(|(i,_)| i).unwrap()
  }

  fn index_of_visible(&self, window: Window) -> uint {
    self.visible.iter().enumerate().find(|&(_,&w)| w == window).map(|(i,_)| i).unwrap()
  }

  fn index_of_hidden(&self, window: Window) -> uint {
    self.hidden.iter().enumerate().find(|&(_,&w)| w == window).map(|(i,_)| i).unwrap()
  }

  fn hide(&mut self, window: Window) {
    let index = self.index_of_visible(window);
    self.visible.remove(index);
    self.hidden.push(window);
  }

  fn remove(&mut self, index: uint) {
    if index < self.hidden.len() {
      self.hidden.remove(index);
    } else {
      self.visible.remove(index - self.hidden.len());
    }
  }
}

pub struct WorkspaceConfig {
  pub tag: String,
  pub screen: uint,
  pub layout: LayoutBox
}

struct Workspace {
  managed: Stack,
  unmanaged: Stack,
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
    if !ws.is_window_floating(window) {
      self.managed.visible.push(window);

      if self.unmanaged.len() > 0 {
        ws.restack_windows(self.all_mut());
      }
    } else {
      self.unmanaged.visible.push(window);
    }

    self.focus_window(ws, config, window);
    if self.visible {
      self.redraw(ws, config);
      ws.map_window(window);
    }
  }

  fn all(&self) -> Vec<Window> {
    self.managed.all().iter().chain(self.unmanaged.all().iter()).map(|&x| x).collect()
  }

  fn all_mut(&mut self) -> Vec<Window> {
    self.managed.all_mut().iter_mut().chain(self.unmanaged.all_mut().iter_mut()).map(|&x| x).collect()
  }

  fn all_visible(&self) -> Vec<Window> {
    self.managed.visible.iter().chain(self.unmanaged.visible.iter()).map(|&x| x).collect()
  }

  pub fn get_layout(&self) -> &Box<Layout + 'static> {
    &self.layout
  }

  pub fn get_tag(&self) -> String {
    self.tag.clone()
  }

  pub fn focused_window(&self) -> Window {
    if self.unmanaged.focused_window == 0 {
      self.managed.focused_window
    } else {
      self.unmanaged.focused_window
    }
  }

  fn remove_managed(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    let index = self.managed.index_of_visible(window);

    self.managed.focused_window = 0;
    ws.unmap_window(window);
    self.managed.remove(index);

    let new_focused_window = if !self.managed.visible.is_empty() {
      self.managed.visible[if index < self.managed.visible.len() { index } else { index - 1}]
    } else {
      0
    };

    if self.visible {
      self.redraw(ws, config);
      self.focus_window(ws, config, new_focused_window);
    }
  }

  fn remove_unmanaged(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    let index = self.unmanaged.index_of_visible(window);

    self.unmanaged.focused_window = 0;
    ws.unmap_window(window);
    self.unmanaged.remove(index);

    let new_focused_window = if !self.unmanaged.visible.is_empty() {
      self.unmanaged.visible[if index < self.unmanaged.len() { index } else { index - 1}]
    } else {
      if !self.managed.visible.is_empty() {
        self.managed.visible[self.managed.len() - 1]
      } else {
        0
      }
    };

    if self.visible {
      self.redraw(ws, config);
      self.focus_window(ws, config, new_focused_window);
    }
  }

  pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    if self.managed.contains(window) {
      self.remove_managed(ws, config, window);
    } else {
      if self.unmanaged.contains(window) {
        self.remove_unmanaged(ws, config, window);
      }
    }
  }

  pub fn hide_window(&mut self, window: Window) {
    debug!("Hide Window {}", window);
    if self.managed.contains(window) {
      self.managed.hide(window);
    } else {
      self.unmanaged.hide(window);
    }
  }
/*
  pub fn show_window(&mut self, window: Window) {
    if self.managed.contains(window) {
      self.managed.hide(window):
    } else {
      self.unmanaged.hide(window);
    }
  }
*/
  pub fn focus_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    if window == 0 {
      return;
    }

    self.unfocus_window(ws, config);

    if self.unmanaged.contains(window) {
      self.unmanaged.focused_window = window;
    } else {
      self.managed.focused_window = window;
    }

    ws.focus_window(window, config.border_focus_color);
  }

  pub fn unfocus_window(&mut self, ws: &XlibWindowSystem, config: &Config) {
    let focused_window = self.focused_window();

    if focused_window != 0 {
      ws.set_window_border_color(focused_window, config.border_color);
      
      if self.unmanaged.focused_window == 0 {
        self.managed.focused_window = 0;
      } else {
        self.unmanaged.focused_window = 0;
      }
    }
  }

  pub fn move_focus(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
    let windows : Vec<Window> = self.all_visible();
    let count = windows.len();

    if self.focused_window() == 0 || count < 2 {
      return;
    }

    let index = windows.iter().enumerate().find(|&(_,&w)| w == self.focused_window()).map(|(i,_)| i).unwrap();
    let new_focused_window = match op {
      Up => {
        if index == 0 {
          windows[count - 1]
        } else {
          windows[index - 1]
        }
      },
      Down => {
        windows[(index + 1) % count]
      },
      Swap => {
        windows[0]
      }
    };

    self.focus_window(ws, config, new_focused_window);
  }

  pub fn move_window(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
    let focused_window = self.focused_window();

    if focused_window == 0 || self.unmanaged.focused_window != 0 {
      return;
    }

    let pos = self.managed.index_of_visible(focused_window);
    let new_pos = match op {
      Up => {
        if pos == 0 {
          self.managed.visible.len() - 1
        } else {
          pos - 1
        }
      },
      Down => {
        (pos + 1) % self.managed.visible.len()
      },
      Swap => {
        let master = self.managed.visible[0];
        self.managed.visible.insert(pos, master);
        self.managed.visible.remove(0);
        0
      }
    };

    self.managed.visible.remove(pos);
    self.managed.visible.insert(new_pos, focused_window);

    self.redraw(ws, config);
  }

  pub fn contains(&self, window: Window) -> bool {
    self.all().iter().any(|&w| w == window)
  }

  pub fn unfocus(&mut self, ws: &XlibWindowSystem, config: &Config) {
    ws.set_window_border_color(self.focused_window(), config.border_color);
  }

  pub fn focus(&self, ws: &XlibWindowSystem, config: &Config) {
    if self.focused_window() != 0 {
      ws.focus_window(self.focused_window(), config.border_focus_color);
    }
  }

  pub fn hide(&mut self, ws: &XlibWindowSystem) {
    self.visible = false;

    for &w in self.managed.visible.iter() {
      ws.unmap_window(w);
    }

    for &w in self.unmanaged.visible.iter() {
      ws.unmap_window(w);
    }
  }

  pub fn show(&mut self, ws: &XlibWindowSystem, config: &Config) {
    self.visible = true;

    self.redraw(ws, config);
    for &w in self.managed.visible.iter() {
      ws.map_window(w);
    }

    for &w in self.unmanaged.visible.iter() {
      ws.map_window(w);
    }
  }

  pub fn redraw(&self, ws: &XlibWindowSystem, config: &Config) {
    debug!("Redraw");
    let screen = ws.get_screen_infos()[self.screen];

    for (i,rect) in self.layout.apply(screen, &self.managed.visible).iter().enumerate() {
      ws.setup_window(rect.x, rect.y, rect.width, rect.height, config.border_width, config.border_color, self.managed.visible[i]);
    }

    for &window in self.unmanaged.visible.iter() {
      let mut rect = ws.get_geometry(window);
      rect.width = rect.width + (2 * config.border_width);
      rect.height = rect.height + (2 * config.border_width);

      ws.setup_window((screen.width - rect.width) / 2, (screen.height - rect.height) / 2, rect.width, rect.height, config.border_width, config.border_color, window);
    }
    self.focus(ws, config);
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
          managed: Stack::new(),
          unmanaged: Stack::new(),
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

  pub fn get(&self, index: uint) -> &Workspace {
    if index < self.list.len() {
      self.list.get(index).unwrap()
    } else {
      self.current()
    }
  }

  pub fn get_mut(&mut self, index: uint) -> &mut Workspace {
    if index < self.list.len() {
      self.list.get_mut(index).unwrap()
    } else {
      self.current_mut()
    }
  }

  pub fn current(&self) -> &Workspace {
    self.list.get(self.cur).unwrap()
  }

  pub fn current_mut(&mut self) -> &mut Workspace {
    self.list.get_mut(self.cur).unwrap()
  }

  pub fn all(&self) -> &Vec<Workspace> {
    &self.list
  }

  pub fn get_index(&self) -> uint {
    self.cur
  }

  pub fn contains(&self, window: Window) -> bool {
    self.list.iter().any(|ws| ws.contains(window))
  }

  pub fn focus_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
    match self.list.iter().enumerate().find(|&(_,workspace)| workspace.contains(window)).map(|(i,_)| i) {
      Some(index) => {
        if self.cur != index {
          self.list[index].focus_window(ws, config, window);
          self.switch_to(ws, config, index);
        } else {
          self.current_mut().focus_window(ws, config, window);
        }
      },
      None => {}
    }
  }

  pub fn switch_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: uint) {
    if self.cur != index && index < self.list.len() {
      if self.list[index].visible {
        if config.greedy_view {
          self.switch_screens(index);
          self.list[self.cur].show(ws, config);
        }
      } else {
        self.list[index].screen = self.list[self.cur].screen;
        self.list[self.cur].hide(ws);
      }

      self.list[self.cur].unfocus(ws, config);
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
    let window = self.list[self.cur].focused_window();
    if window == 0 {
      return;
    }

    self.remove_window(ws, config, window);
    self.list[index].add_window(ws, config, window);
    self.list[index].unfocus(ws, config);
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
    match self.find_window(window) {
      Some(workspace) => {
        workspace.remove_window(ws, config, window);
      },
      None => {}
    }
  }

  pub fn hide_window(&mut self, window: Window) {
    if let Some(workspace) = self.find_window(window) {
      workspace.hide_window(window);
    }
  }

  pub fn rescreen(&mut self, ws: &XlibWindowSystem, config: &Config) {
    let new_screens = ws.get_screen_infos().len();
    let prev_screens = self.list.iter().fold(0, |acc, x| cmp::max(acc, x.screen));

    // move and hide workspaces if their screens got removed
    for workspace in self.list.iter_mut().filter(|ws| ws.screen > (new_screens - 1)) {
      workspace.screen = 0;
      workspace.hide(ws);
    }

    // assign the first hidden workspace to the new screen
    for screen in range(prev_screens, new_screens) {
      match self.list.iter_mut().find(|ws| !ws.visible) {
        Some(workspace) => {
          workspace.screen = screen;
          workspace.show(ws, config);
        },
        None => {
          break;
        }
      }
    }
  }

  fn find_window(&mut self, window: Window) -> Option<&mut Workspace> {
    self.list.iter_mut().find(|workspace| workspace.contains(window))
  }

  fn switch_screens(&mut self, dest: uint) {
    let screen = self.list[self.cur].screen;
    self.list[self.cur].screen = self.list[dest].screen;
    self.list[dest].screen = screen;
  }
}