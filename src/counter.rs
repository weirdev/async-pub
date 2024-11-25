// Adapt the generic logger for use as a count publisher

use std::collections::HashMap;

use crate::logger::{Logger, Publisher};

struct CountersStruct(Logger<String>);
static Counters: CountersStruct = CountersStruct(Logger::new::<CounterPublishState>());

struct CounterPublishState {
    counters: HashMap<String, usize>,
}

impl CounterPublishState {
    fn publish_to_remote(&self, counter: String, count: usize) {
        println!("{}: {}", counter, count);
    }
}

impl Publisher<String> for CounterPublishState {
    fn new() -> Self {
        CounterPublishState {
            counters: HashMap::new(),
        }
    }

    fn send(&mut self, counter: String) -> Result<(), ()> {
        let cnt: usize;
        if let Some(count) = self.counters.get_mut(&counter) {
            *count += 1;
            cnt = *count;
        } else {
            self.counters.insert(counter.clone(), 1);
            cnt = 1;
        }

        if cnt == cnt.next_power_of_two() {
            self.publish_to_remote(counter, cnt);
        }

        Ok(())
    }
}

impl Drop for CounterPublishState {
    fn drop(&mut self) {
        if self.counters.values().any(|x| *x > 0) {
            panic!("Some counters not published");
        }
    }
}

pub fn incCounter(counter: String) {
    Counters.0.send(counter).unwrap();
}
