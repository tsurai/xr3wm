use std::old_io::{File, USER_RWX};
use std::old_io::fs::{PathExtensions, mkdir};

fn main() {
    let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/config.rs"));
    if !dst.exists() {
      mkdir(&dst.dir_path(), USER_RWX);
      let mut f = File::create(&dst).unwrap();
      f.write_str(
"pub fn get_config<'a>() -> Config<'a> {
  Default::default()
}").unwrap();
    }
}