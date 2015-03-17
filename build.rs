use std::io::prelude::*;
use std::path::Path;
use std::fs::{File, create_dir};

fn main() {
    let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/config.rs"));
    if !dst.exists() {
      match create_dir(dst) {
        Ok(_) => {
          let mut f = File::create(&dst).unwrap();
          f.write_all(
b"pub fn get_config<'a>() -> Config<'a> {
  Default::default()
}").unwrap();
        },
        Err(msg) => {
          panic!("Failed to create config directory: {}", msg);
        }
      }
    }
}

