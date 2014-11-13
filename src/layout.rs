use xlib::Window;
use std::rc::Rc;

#[deriving(Clone)]
pub struct Rect {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32
}

pub trait Layout {
  fn apply(&self, Rect, &Vec<Window>) -> Vec<Rect>;
}

pub fn to_box<T: Layout + 'static>(layout: T) -> Rc<Box<Layout + 'static>> {
  Rc::new(box layout as Box<Layout>)
}

pub struct TallLayout {
  num_masters: u8,
  ratio: f32,
  ratio_increment: f32
}

impl TallLayout {
  pub fn new(num_masters: u8, ratio: f32, ratio_increment: f32) -> TallLayout {
    TallLayout {
      num_masters: num_masters,
      ratio: ratio,
      ratio_increment: ratio_increment,
    }
  }
}

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