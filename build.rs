use std::io;
use std::io::File;
use std::io::fs;
use std::io::fs::PathExtensions;

fn main() {
    let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/config.rs"));
    if !dst.exists() {
      fs::mkdir(&dst.dir_path(), io::USER_RWX);
      let mut f = File::create(&dst).unwrap();
      f.write_str(
"pub fn get_config<'a>() -> Config<'a> {
  Default::default()
}").unwrap();
    }
}