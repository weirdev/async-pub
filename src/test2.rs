use crate::logger;
use logger::UnsafeSyncCell;

static BGD: UnsafeSyncCell<Vec<String>> = UnsafeSyncCell::new(Vec::new());

pub fn main() {
    unsafe {
        BGD.data.get().as_mut().unwrap().push("Hello, world!".to_string());
        println!("{}", BGD.data.get().as_mut().unwrap().join(", "));
        BGD.data.get().as_mut().unwrap().push("Hello, again!".to_string());
        println!("{}", BGD.data.get().as_mut().unwrap().join(", "));
    }
}
