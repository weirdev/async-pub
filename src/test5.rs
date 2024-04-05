use std::{thread, time};
use rand::{rngs::ThreadRng, Rng};

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

fn sleep_random_millis(rng: &mut ThreadRng) {
    let millis = rng.gen_range(0..21);  // Generates a number between 0 and 20
    let duration = time::Duration::from_millis(millis);
    thread::sleep(duration);
}

pub fn main() {
    let rx = BGD.set_new_channel();

    let receiver = thread::spawn(move || {
        // Because we keep a live sender at all times, iterating
        // past the 10th element will block the program
        for out in rx.iter().take(10) {
            println!("{}", out);
        }
    });

    for i in 0..10 {
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            sleep_random_millis(&mut rng);
            let _ = BGD.send(i);
        });
    }

    receiver.join().unwrap();
}
