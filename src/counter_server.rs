use core::panic;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, RwLock},
};

use futures::prelude::*;
use tokio::net::TcpListener;
use tokio_serde::formats::*;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use crate::counter_types::{get_epoc_minutes, CounterMessage, CounterState, CounterUpdateMessage};

struct TimeBucketSpec {
    interval_minutes: u64,
    interval_count: usize,
}

const TIME_BUCKET_COUNT: usize = 5;
/// Newest to oldest
type TimeSeries = [TimeBucket; TIME_BUCKET_COUNT];
const TIME_BUCKET_SPECS: [TimeBucketSpec; TIME_BUCKET_COUNT] = [
    // 6 hours of 1 minute resolution
    // Through 6 hours
    TimeBucketSpec {
        interval_minutes: 1,
        interval_count: 6 * 60,
    },
    // 18 hours of 5 minute resolution
    // Through 24 hours
    TimeBucketSpec {
        interval_minutes: 5,
        interval_count: 18 * (60 / 5),
    },
    // 5 days of 15 minute resolution
    // Through 7 days
    TimeBucketSpec {
        interval_minutes: 15,
        interval_count: 5 * 24 * (60 / 15),
    },
    // 23 days of 1 hour resolution
    // Through 30 days
    TimeBucketSpec {
        interval_minutes: 60,
        interval_count: 23 * 24,
    },
    // 60 days of 3 hour resolution
    // Through 90 days
    TimeBucketSpec {
        interval_minutes: 3 * 60,
        interval_count: 60 * (24 / 3),
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimeBucket {
    /// Minutes per interval
    interval_minutes: u64,
    /// This bucket holds [previous cutoff_minutes, cutoff_minutes) minutes of data
    cutoff_minutes: u64,
    /// Oldest to newest
    data: VecDeque<CounterState>,
}

#[tokio::main]
pub async fn run_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;

    let counters: HashMap<String, Arc<Mutex<[TimeBucket; 5]>>> = HashMap::new();
    let counters = Arc::new(RwLock::new(counters));
    // let time_series = Arc::new(Mutex::new(time_series));

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();

        let counters = counters.clone();

        // For each client, spawn a new task
        tokio::spawn(async move {
            let (reader, mut _writer) = socket.split();

            let length_delimited = FramedRead::new(reader, LengthDelimitedCodec::new());

            // Deserialize frames
            let mut deserialized = tokio_serde::SymmetricallyFramed::new(
                length_delimited,
                SymmetricalJson::<CounterMessage>::default(),
            );

            // We could process each message on its own thread, but that is probably not efficient
            while let Some(msg) = deserialized.try_next().await.unwrap() {
                // Must get the time after the message is received
                let server_epoch_minutes = get_epoc_minutes();

                match msg {
                    CounterMessage::Read(counter) => {
                        let time_series = counters.read().unwrap();
                        if let Some(time_series) = time_series.get(&counter) {
                            let time_series = time_series.lock().unwrap();
                            time_series.iter().flat_map(|bucket| bucket.data.iter()).for_each(
                                |counter_state| {
                                    println!(
                                        "{} [{}]: {}",
                                        counter, counter_state.epoch_minutes, counter_state.count
                                    );
                                },
                            );
                        }
                    }
                    CounterMessage::Update(msg) => {
                        msg.state.iter().for_each(|counter_state| {
                            println!(
                                "{} [{}]: {}",
                                msg.counter, counter_state.epoch_minutes, counter_state.count
                            );
                        });
        
                        // Get the time series for the counter, creating it if it doesn't exist
                        let time_series =
                            if let Some(time_series) = counters.read().unwrap().get(&msg.counter) {
                                time_series.clone()
                            } else {
                                let time_series = Arc::new(Mutex::new(create_time_series()));
                                counters
                                    .write()
                                    .unwrap()
                                    .insert(msg.counter.clone(), time_series.clone());
                                time_series
                            };
        
                        update_time_series(
                            &mut *time_series.lock().unwrap(),
                            msg.state,
                            server_epoch_minutes,
                        );
                    }
                }
            }
        });
    }
}

fn update_time_series(
    time_series: &mut TimeSeries,
    counter_message: Vec<CounterState>,
    server_epoch_minutes: u64,
) {
    // TODO: Stop accepting messages that are too much older than the server time?
    // For now, reject messages that are older than the youngest time series bucket

    // Update the existing time series buckets, so that none hold data past thier cutoff
    shift_time_series(time_series, server_epoch_minutes);

    counter_message.into_iter().for_each(|counter_state| {
        add_to_series(time_series, counter_state, server_epoch_minutes);
    });
}

/// Update the existing time series buckets, so that none hold data past thier cutoff
fn shift_time_series(time_series: &mut TimeSeries, server_epoch_minutes: u64) {
    for i in (0..time_series.len()).rev() {
        // Oldest to earliest bucket
        let (younger, older) = time_series.split_at_mut(i + 1);
        let bucket = younger.last_mut().unwrap();
        // Look at the oldest interval
        while bucket.data.front_mut().map_or(false, |last_state| {
            bucket.cutoff_minutes < server_epoch_minutes - last_state.epoch_minutes
        }) {
            let graduated_state = bucket.data.pop_back().unwrap();
            // We have one or more intervals that are too old for bucket
            add_to_series(older, graduated_state, server_epoch_minutes);
        }
    }
}

fn add_to_series(
    time_series: &mut [TimeBucket],
    counter_state: CounterState,
    server_epoch_minutes: u64,
) {
    if time_series.is_empty() {
        // Happens when the counter state is too old for any time series bucket
        return;
    }

    if time_series[0].cutoff_minutes < server_epoch_minutes - counter_state.epoch_minutes {
        // The counter state is too old for the current time series bucket
        add_to_series(&mut time_series[1..], counter_state, server_epoch_minutes);
        return;
    }

    // Most often the counter state will be added to the newest bucket
    let bucket = &mut time_series[0];
    add_to_bucket(bucket, counter_state, server_epoch_minutes);
}

fn add_to_bucket(
    bucket: &mut TimeBucket,
    mut counter_state: CounterState,
    server_epoch_minutes: u64,
) {
    if server_epoch_minutes - counter_state.epoch_minutes > bucket.cutoff_minutes {
        panic!("Counter state is too old for the bucket");
    }

    for i in (0..bucket.data.len()).rev() {
        // Handle updates of any age, but generally expect the newest state to be updated
        // i.e. Return after the first iteration
        let interval_state = &mut bucket.data[i];
        if counter_state.epoch_minutes < interval_state.epoch_minutes {
            // New state comes before the current interval
            // Give the new counter state an aligned minute value
            counter_state.epoch_minutes -= counter_state.epoch_minutes % bucket.interval_minutes;
            if i == bucket.data.len() - 1 {
                // The new state will be the newest in the bucket
                bucket.data.push_back(counter_state);
                return;
            } else {
                bucket.data.insert(i + 1, counter_state);
            }
            return;
        } else if interval_state.epoch_minutes + bucket.interval_minutes
            > counter_state.epoch_minutes
        {
            // New state comes falls into the current interval
            interval_state.count += counter_state.count;
            return;
        }
    }
    counter_state.epoch_minutes -= counter_state.epoch_minutes % bucket.interval_minutes;
    // The new state will be the oldest in the bucket
    bucket.data.push_front(counter_state);
}

fn create_time_series() -> TimeSeries {
    let mut series_vec = Vec::with_capacity(TIME_BUCKET_COUNT);
    let mut cutoff = 0;
    for spec in TIME_BUCKET_SPECS.iter() {
        cutoff += spec.interval_minutes * spec.interval_count as u64;
        series_vec.push(TimeBucket {
            interval_minutes: spec.interval_minutes,
            cutoff_minutes: cutoff,
            data: VecDeque::with_capacity(spec.interval_count),
        });
    }

    <TimeSeries>::try_from(series_vec)
        .unwrap_or_else(|_| panic!("Failed to create time series buckets"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_series_creation() {
        let ts = create_time_series();
        assert_eq!(ts.len(), TIME_BUCKET_COUNT);
        assert_eq!(ts[0].interval_minutes, 1);
        assert_eq!(ts[1].interval_minutes, 5);
        assert_eq!(ts[2].interval_minutes, 15);
        assert_eq!(ts[3].interval_minutes, 60);
        assert_eq!(ts[4].interval_minutes, 3 * 60);

        assert_eq!(ts[0].cutoff_minutes, 6 * 60);
        assert_eq!(ts[1].cutoff_minutes, (6 * 60) + (18 * 60));
        assert_eq!(ts[2].cutoff_minutes, (6 * 60) + (18 * 60) + (5 * 24 * 60));
        assert_eq!(
            ts[3].cutoff_minutes,
            (6 * 60) + (18 * 60) + (5 * 24 * 60) + (23 * 24 * 60)
        );
        assert_eq!(
            ts[4].cutoff_minutes,
            (6 * 60) + (18 * 60) + (5 * 24 * 60) + (23 * 24 * 60) + (3 * 60 * 8 * 60)
        );

        assert_eq!(ts[0].data.capacity(), 6 * 60);
        assert_eq!(ts[1].data.capacity(), 18 * 12);
        assert_eq!(ts[2].data.capacity(), 5 * 24 * 4);
        assert_eq!(ts[3].data.capacity(), 23 * 24);
        assert_eq!(ts[4].data.capacity(), 60 * 8);
    }

    #[test]
    fn add_to_empty_1min_bucket() {
        let mut bucket = TimeBucket {
            interval_minutes: 1,
            cutoff_minutes: 6 * 60,
            data: VecDeque::with_capacity(6 * 60),
        };

        let counter_state = CounterState {
            epoch_minutes: 5,
            count: 3,
        };

        add_to_bucket(&mut bucket, counter_state, 6);

        assert_eq!(bucket.data.len(), 1);
        assert_eq!(bucket.data[0].epoch_minutes, 5);
        assert_eq!(bucket.data[0].count, 3);
    }

    #[test]
    fn add_to_empty_1min_bucket_multiple() {
        let mut bucket = TimeBucket {
            interval_minutes: 1,
            cutoff_minutes: 6 * 60,
            data: VecDeque::with_capacity(6 * 60),
        };

        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_bucket(&mut bucket, counter_state, 1);

        assert_eq!(bucket.data.len(), 1);
        assert_eq!(bucket.data[0].epoch_minutes, 1);
        assert_eq!(bucket.data[0].count, 1);

        let counter_state = CounterState {
            epoch_minutes: 4,
            count: 1,
        };

        add_to_bucket(&mut bucket, counter_state, 4);

        assert_eq!(bucket.data.len(), 2);
        assert_eq!(bucket.data[0].epoch_minutes, 4);
        assert_eq!(bucket.data[0].count, 1);
        assert_eq!(bucket.data[1].epoch_minutes, 1);
        assert_eq!(bucket.data[1].count, 1);

        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_bucket(&mut bucket, counter_state, 4);

        assert_eq!(bucket.data.len(), 2);
        assert_eq!(bucket.data[0].epoch_minutes, 4);
        assert_eq!(bucket.data[0].count, 1);
        assert_eq!(bucket.data[1].epoch_minutes, 1);
        assert_eq!(bucket.data[1].count, 2);
    }

    #[test]
    fn add_to_empty_3minute_bucket_multiple() {
        let mut bucket = TimeBucket {
            interval_minutes: 3,
            cutoff_minutes: 6 * 60,
            data: VecDeque::with_capacity(6 * 60 / 3),
        };

        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_bucket(&mut bucket, counter_state, 1);

        assert_eq!(bucket.data.len(), 1);
        // 1 min in the 3 min bucket goes into the 0-2 min interval
        assert_eq!(bucket.data[0].epoch_minutes, 0);
        assert_eq!(bucket.data[0].count, 1);

        let counter_state = CounterState {
            epoch_minutes: 4,
            count: 1,
        };

        add_to_bucket(&mut bucket, counter_state, 4);

        assert_eq!(bucket.data.len(), 2);
        assert_eq!(bucket.data[0].epoch_minutes, 3);
        assert_eq!(bucket.data[0].count, 1);
        assert_eq!(bucket.data[1].epoch_minutes, 0);
        assert_eq!(bucket.data[1].count, 1);

        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_bucket(&mut bucket, counter_state, 4);

        assert_eq!(bucket.data.len(), 2);
        assert_eq!(bucket.data[0].epoch_minutes, 3);
        assert_eq!(bucket.data[0].count, 1);
        assert_eq!(bucket.data[1].epoch_minutes, 0);
        assert_eq!(bucket.data[1].count, 2);

        let counter_state = CounterState {
            epoch_minutes: 5,
            count: 7,
        };

        add_to_bucket(&mut bucket, counter_state, 5);

        assert_eq!(bucket.data.len(), 2);
        assert_eq!(bucket.data[0].epoch_minutes, 3);
        assert_eq!(bucket.data[0].count, 8);
        assert_eq!(bucket.data[1].epoch_minutes, 0);
        assert_eq!(bucket.data[1].count, 2);
    }

    #[test]
    fn add_to_0len_series() {
        let mut series = Vec::new();
        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_series(&mut series, counter_state, 1);

        assert_eq!(series.len(), 0);
    }

    #[test]
    fn add_to_standard_series() {
        let mut series = create_time_series();
        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_series(&mut series, counter_state, 1);

        assert_eq!(series[0].data.len(), 1);
        assert_eq!(series[0].data[0].epoch_minutes, 1);
        assert_eq!(series[0].data[0].count, 1);
    }

    #[test]
    fn add_to_standard_series_multiple_first_bucket() {
        let mut series = create_time_series();
        let counter_state = CounterState {
            epoch_minutes: 1,
            count: 1,
        };

        add_to_series(&mut series, counter_state, 1);

        assert_eq!(series[0].data.len(), 1);
        assert_eq!(series[0].data[0].epoch_minutes, 1);
        assert_eq!(series[0].data[0].count, 1);

        let counter_state = CounterState {
            epoch_minutes: 2,
            count: 1,
        };

        add_to_series(&mut series, counter_state, 2);

        assert_eq!(series[0].data.len(), 2);
        assert_eq!(series[0].data[0].epoch_minutes, 2);
        assert_eq!(series[0].data[0].count, 1);
        assert_eq!(series[0].data[1].epoch_minutes, 1);
        assert_eq!(series[0].data[1].count, 1);
    }

    #[test]
    fn add_to_standard_series_multiple_multiple_buckets() {
        const START_POINT: u64 = 18 * 60;

        let mut series = create_time_series();
        let counter_state = CounterState {
            epoch_minutes: START_POINT,
            count: 1,
        };

        add_to_series(&mut series, counter_state, START_POINT);

        assert_eq!(series[0].data.len(), 1);
        assert_eq!(series[0].data[0].epoch_minutes, START_POINT);
        assert_eq!(series[0].data[0].count, 1);
        assert!(series[1..].iter().all(|bucket| bucket.data.is_empty()));

        let counter_state = CounterState {
            epoch_minutes: START_POINT + 1,
            count: 2,
        };

        add_to_series(&mut series, counter_state, START_POINT + 1);

        assert_eq!(series[0].data.len(), 2);
        assert_eq!(series[0].data[0].epoch_minutes, START_POINT + 1);
        assert_eq!(series[0].data[0].count, 2);
        assert_eq!(series[0].data[1].epoch_minutes, START_POINT);
        assert_eq!(series[0].data[1].count, 1);
        assert!(series[1..].iter().all(|bucket| bucket.data.is_empty()));

        let counter_state = CounterState {
            epoch_minutes: 0, // Much older than the first bucket
            count: 5,
        };

        add_to_series(&mut series, counter_state, START_POINT + 1);

        assert_eq!(series[0].data.len(), 2);
        assert_eq!(series[0].data[0].epoch_minutes, START_POINT + 1);
        assert_eq!(series[0].data[0].count, 2);
        assert_eq!(series[0].data[1].epoch_minutes, START_POINT);
        assert_eq!(series[0].data[1].count, 1);
        assert_eq!(series[1].data.len(), 1);
        assert_eq!(series[1].data[0].epoch_minutes, 0);
        assert_eq!(series[1].data[0].count, 5);
        assert!(series[2..].iter().all(|bucket| bucket.data.is_empty()));
    }

    #[test]
    fn add_to_standard_series_past_final_cutoff() {
        let mut series = create_time_series();
        let start_point = series[TIME_BUCKET_COUNT - 1].cutoff_minutes + 100;
        let counter_state = CounterState {
            epoch_minutes: 10,
            count: 1,
        };

        add_to_series(&mut series, counter_state, start_point);

        assert!(series.iter().all(|bucket| bucket.data.is_empty()));
    }

    #[test]
    fn shift_time_series_no_shift() {
        const START_POINT: u64 = 18 * 60;

        let mut series = create_time_series();
        let counter_state = CounterState {
            epoch_minutes: 10,
            count: 2,
        };

        add_to_series(&mut series, counter_state, START_POINT);

        let counter_state = CounterState {
            epoch_minutes: 11,
            count: 3,
        };

        add_to_series(&mut series, counter_state, START_POINT + 1);

        let pre_shift = series.clone();

        shift_time_series(&mut series, START_POINT);

        assert_eq!(series, pre_shift);
    }

    #[test]
    fn shift_time_series_shift_multiple_to_one() {
        const START_POINT: u64 = 18 * 60;

        let mut series = create_time_series();
        let counter_state = CounterState {
            epoch_minutes: 10,
            count: 2,
        };

        add_to_series(&mut series, counter_state, START_POINT);

        let counter_state = CounterState {
            epoch_minutes: 11,
            count: 3,
        };

        add_to_series(&mut series, counter_state, START_POINT + 1);

        assert_eq!(series[0].data.len(), 0);
        assert_eq!(series[1].data.len(), 1);
        assert_eq!(
            series[2..].iter().all(|bucket| bucket.data.is_empty()),
            true
        );

        let pre_shift = series.clone();

        // One day later
        shift_time_series(&mut series, START_POINT + (60 * 24));

        assert_ne!(series, pre_shift);
        assert_eq!(series[0].data.len(), 0);
        assert_eq!(series[1].data.len(), 0);
        assert_eq!(series[2].data.len(), 1);
        assert_eq!(series[2].data[0].epoch_minutes, 0);
        assert_eq!(series[2].data[0].count, 5);
        assert!(series[3..].iter().all(|bucket| bucket.data.is_empty()));
    }

    #[test]
    fn shift_time_series_shift_multiple_to_multiple() {
        const START_POINT: u64 = 18 * 60;

        let mut series = create_time_series();
        let counter_state = CounterState {
            epoch_minutes: 10,
            count: 2,
        };

        add_to_series(&mut series, counter_state, START_POINT);

        let counter_state = CounterState {
            epoch_minutes: START_POINT - 10,
            count: 3,
        };

        add_to_series(&mut series, counter_state, START_POINT);

        assert_eq!(series[0].data.len(), 1);
        assert_eq!(series[1].data.len(), 1);
        assert_eq!(series[2..].iter().all(|bucket| bucket.data.is_empty()), true);

        let pre_shift = series.clone();

        // 18 hours later
        shift_time_series(&mut series, START_POINT + (60 * 18));

        assert_ne!(series, pre_shift);
        assert_eq!(series[0].data.len(), 0);
        assert_eq!(series[1].data.len(), 1);
        assert_eq!(series[1].data[0].epoch_minutes, 1070);
        assert_eq!(series[1].data[0].count, 3);
        assert_eq!(series[2].data.len(), 1);
        assert_eq!(series[2].data[0].epoch_minutes, 0);
        assert_eq!(series[2].data[0].count, 2);
        assert_eq!(series[3..].iter().all(|bucket| bucket.data.is_empty()), true);
    }
}
