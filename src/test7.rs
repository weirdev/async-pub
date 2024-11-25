use std::{thread, time};

use rand::{rngs::ThreadRng, Rng};

use crate::counter::incCounter;

fn sleep_random_millis(rng: &mut ThreadRng) {
    let millis = rng.gen_range(0..21); // Generates a number between 0 and 20
    let duration = time::Duration::from_millis(millis);
    thread::sleep(duration);
}

pub fn main() {
    for i in 0..32 {
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            sleep_random_millis(&mut rng);
            incCounter("did_a_thing".to_string());
            incCounter(format!("did_a_new_thing_{}", i));
        });
    }
    
    thread::sleep(time::Duration::from_secs(1));
}