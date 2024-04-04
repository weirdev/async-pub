use crate::bg;
use bg::UnsafeSyncCell;

static BGD: UnsafeSyncCell<String> = UnsafeSyncCell::new(String::new());

pub fn main() {
    unsafe {
        let _ = std::mem::replace(
            BGD.data.get().as_mut().unwrap(),
            "Hello, world!".to_string(),
        );
        println!("{}", BGD.data.get().as_ref().unwrap());
    }
}
