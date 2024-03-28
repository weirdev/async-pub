mod bg;

use std::cell::UnsafeCell;

use bg::{BGData, UnsafeSyncCell};

static BGD: BGData<UnsafeSyncCell<String>, String, String> = BGData::new(UnsafeSyncCell {
    data: UnsafeCell::new(String::new()),
});

fn main() {
    BGD.write("Hello, world!".to_string());
    println!("{}", BGD.read());
}
