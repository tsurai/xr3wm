extern crate serialize;

use serialize::json;
use serialize::hex::{FromHex, InvalidHexCharacter, InvalidHexLength};
use std::io::{File, Open, ReadWrite};
use std::default::Default;

#[deriving(Decodable,Clone)]
pub struct Config {
  pub mod_key: String,
  pub border_width: uint,
  pub border_color: String,
  pub border_focus_color: String,
  pub terminal: String
}

impl Default for Config {
  fn default() -> Config {
    Config{
      mod_key: String::from_str("mod4"),
      border_width: 1,
      border_color: String::from_str("00ff0000"),
      border_focus_color: String::from_str("0000ff00"),
      terminal: String::from_str("xterm")
    }
  }
}

impl Config {
  pub fn load(file: String) -> Config {
    match File::open_mode(&Path::new(file), Open, ReadWrite) {
      Ok(mut f) => {
        match f.read_to_string() {
          Ok(fstring) => {
            match json::decode(fstring.as_slice()) {
              Ok(config) => config,
              Err(err) => {
                // TODO: proper error output
                Default::default()
              }
            }
          },
          Err(err) => {
            // TODO: proper error output
            Default::default()
          }
        }
      },
      Err(err) => {
        // TODO: proper error output
        Default::default()
      }
    }
  }

  pub fn get_border_color_as_u64(&self) -> u64 {
    hex_to_u64(&self.border_color)
  }

  pub fn get_border_focus_color_as_u64(&self) -> u64 {
    hex_to_u64(&self.border_focus_color)
  }
}

fn hex_to_u64(hex_str: &String) -> u64 {
  let bytes = match hex_str.as_slice().from_hex() {
    Ok(hex) => {
      hex
    },
    Err(err) => {
      match err {
        InvalidHexCharacter(char, uint) => {
          // TODO: proper error output
        },
        InvalidHexLength => {
          // TODO: proper error output
        }
      }
      let def : Config = Default::default();
      def.border_color.as_slice().from_hex().unwrap()
    }
  };
  bytes.iter().enumerate().fold(0 as u64, |a, (i,&b)| a + (b as u64 << 8 * ((bytes.len() - 1) - i))) as u64
}
