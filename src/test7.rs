use std::{thread, time};

use rand::{rngs::ThreadRng, Rng};

use crate::counter::inc_counter;

fn sleep_random_millis(rng: &mut ThreadRng) {
    let millis = rng.gen_range(0..72000); // Generates a number between 0 and 20
    let duration = time::Duration::from_millis(millis);
    thread::sleep(duration);
}

pub fn main() {
    for i in 0..330 {
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            sleep_random_millis(&mut rng);
            inc_counter("did_a_thing".to_string());
            if i % 10 == 0 {
                inc_counter(format!("did_a_new_thing_{}", i));
            }
        });
    }

    thread::sleep(time::Duration::from_secs(70));
}
