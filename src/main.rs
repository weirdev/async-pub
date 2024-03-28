mod bg;

use bg::{BGData, UnsafeSyncCell};

static BGD: BGData<UnsafeSyncCell<String>, String, String> =
    BGData::new(UnsafeSyncCell::new(String::new()));

fn main() {
    BGD.write("Hello, world!".to_string());
    println!("{}", BGD.read());
}
