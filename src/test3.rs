use std::sync::RwLock;

static BGD: RwLock<Vec<u8>> = RwLock::new(Vec::new());

trait Join {
    fn join(self, sep: &str) -> String;
}

impl<I: Iterator<Item = String>> Join for I {
    fn join(self, separator: &str) -> String {
        self.collect::<Vec<_>>().join(separator)
    }
}

pub fn main() {
    BGD.write().unwrap().push(0);
    println!("{}", BGD.read().unwrap().iter().map(|x| x.to_string()).join(", "));
    BGD.write().unwrap().push(1);
    println!("{}", BGD.read().unwrap().iter().map(|x| x.to_string()).join(", "));
}
