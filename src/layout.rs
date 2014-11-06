use xlib::Window;
use xlib_window_system::XlibWindowSystem;

#[deriving(Clone)]
pub struct Rect{
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32
}

pub trait Layout {
  fn apply(&self, Rect, &Vec<Window>) -> Vec<Rect>;
}

pub struct TallLayout {
  num_masters: u32
}

impl TallLayout {
  pub fn new(num_masters: u32) -> Box<TallLayout> {
    box TallLayout{
      num_masters: num_masters
    }
  }
}

// TODO: adjust dimensions to consider the border width
impl Layout for TallLayout {
  fn apply(&self, screen: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    Vec::from_fn(windows.len(), |len| {
      match len + 1 {
        1 => {
          if windows.len() > 1 {
            Rect{x: 0, y: 0, width: screen.width / 2, height: screen.height}
          } else {
            Rect{x: 0, y: 0, width: screen.width, height: screen.height}
          }
        },
        n => {
          let xoff = screen.width / 2;
          let yoff = screen.height / (windows.len() - 1) as u32;

          Rect{x: xoff, y: yoff * (n - 2) as u32, width: xoff, height: yoff}
        }
      }
    })
  }
}