use crate::bg::CommonSender;

static BGD: CommonSender<u8> = CommonSender::new();

trait Join {
    fn join(self, sep: &str) -> String;
}

impl<I: Iterator<Item = String>> Join for I {
    fn join(self, separator: &str) -> String {
        self.collect::<Vec<_>>().join(separator)
    }
}

pub fn main() {
    let rx = BGD.set_new_channel();

    let _ = BGD.send(0);
    println!("{}", rx.recv().unwrap());
    let _ = BGD.send(1);
    println!("{}", rx.recv().unwrap());
}
