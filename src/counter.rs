// Adapt the generic logger for use as a count publisher

use std::collections::HashMap;
use std::time;

use serde::{Deserialize, Serialize};

use crate::logger::{Logger, Publisher};

struct CountersStruct(Logger<String>);
static COUNTERS: CountersStruct = CountersStruct(Logger::new::<CounterPublishState>());

struct CounterPublishState {
    counters: HashMap<String, CounterState>,
}

#[derive(Deserialize, Serialize)]
struct CounterState {
    epoch_minutes: u64,
    count: usize,
}

#[derive(Deserialize, Serialize)]
struct CounterMessage {
    counter: String,
    state: Vec<CounterState>,
}

impl CounterPublishState {
    fn publish_to_remote(
        &self,
        counter: String,
        prev_counter_state: Option<CounterState>,
        cur_counter_state: CounterState,
    ) {
        let mut state = Vec::with_capacity(2);
        if let Some(prev_cs) = prev_counter_state {
            state.push(prev_cs);
        }
        state.push(cur_counter_state);
        let message = CounterMessage { counter, state };
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap()
            .block_on(
                reqwest::Client::new()
                    .post("http://localhost:8080/counter")
                    .body(serde_json::to_string(&message).unwrap())
                    .send(),
            )
            .unwrap();
    }
}

impl Publisher<String> for CounterPublishState {
    fn new() -> Self {
        CounterPublishState {
            counters: HashMap::new(),
        }
    }

    fn send(&mut self, counter: String) -> Result<(), ()> {
        let epoch_seconds = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        // ~1 minute accuracy
        let epoch_minutes = epoch_seconds / 60;

        let mut prev_cs: Option<CounterState> = None;
        let cur_count = if let Some(cs) = self.counters.get_mut(&counter) {
            if cs.epoch_minutes == epoch_minutes {
                cs.count += 1;

                cs.count
            } else {
                prev_cs = Some(std::mem::replace(
                    cs,
                    CounterState {
                        epoch_minutes,
                        count: 1,
                    },
                ));

                1
            }
        } else {
            self.counters.insert(
                counter.clone(),
                CounterState {
                    epoch_minutes,
                    count: 1,
                },
            );

            1
        };

        if prev_cs.is_some() || cur_count == cur_count.next_power_of_two() {
            self.publish_to_remote(
                counter,
                prev_cs,
                CounterState {
                    epoch_minutes,
                    count: cur_count,
                },
            );
        }

        Ok(())
    }
}

impl Drop for CounterPublishState {
    fn drop(&mut self) {
        if self.counters.values().any(|c| c.count > 0) {
            panic!("Some counters not published");
        }
    }
}

pub fn inc_counter(counter: String) {
    COUNTERS.0.send(counter).unwrap();
}
