use rand::{rngs::ThreadRng, Rng};
use std::{thread, time};

use crate::logger::{Logger, Publisher};

static BGD: Logger<u8> = Logger::new::<Pub>();

fn sleep_random_millis(rng: &mut ThreadRng) {
    let millis = rng.gen_range(0..21); // Generates a number between 0 and 20
    let duration = time::Duration::from_millis(millis);
    thread::sleep(duration);
}
struct Pub {
    msg_count: u32,
}

impl Publisher<u8> for Pub {
    fn new() -> Self {
        Pub { msg_count: 0 }
    }

    fn send(&mut self, data: u8) -> Result<(), ()> {
        self.msg_count += 1;
        let duration = time::Duration::from_millis(2000);
        thread::sleep(duration);
        println!("{}", data);
        Ok(())
    }
}

impl Drop for Pub {
    fn drop(&mut self) {
        println!("Pub sent {} messages", self.msg_count);
    }
}

pub fn main() {
    for i in 0..10 {
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            sleep_random_millis(&mut rng);
            let _ = BGD.send(i);
        });
    }

    thread::sleep(time::Duration::from_secs(1));
    BGD.close(); // Block until the logger has published all it's items
}
