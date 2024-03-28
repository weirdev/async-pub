use crate::bg;
use bg::{RAppend, UnsafeSyncCell};

static BGD: UnsafeSyncCell<Vec<String>> = UnsafeSyncCell::new(Vec::new());

pub fn main() {
    BGD.append("Hello, world!".to_string());
    println!("{}", BGD.read().join(", "));
    BGD.append("Hello, again!".to_string());
    println!("{}", BGD.read().join(", "));
}
