use crate::bg;
use bg::{RWReplace, UnsafeSyncCell, RW};

static BGD: UnsafeSyncCell<String> = UnsafeSyncCell::new(String::new());

pub fn main() {
    BGD.replace("Hello, world!".to_string());
    println!("{}", BGD.read());
}
