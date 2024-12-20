use std::time;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CounterState {
    pub epoch_minutes: u64,
    pub count: usize,
}

#[derive(Deserialize, Serialize)]
pub struct CounterUpdateMessage {
    pub counter: String,
    pub state: Vec<CounterState>,
}

#[derive(Deserialize, Serialize)]
pub enum CounterMessage {
    Read(String),
    Update(CounterUpdateMessage),
}

pub fn get_epoc_minutes() -> u64 {
    let epoch_seconds = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    epoch_seconds / 60
}
