use core::{panic, time};
use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};

use futures::prelude::*;
use tokio::net::TcpListener;
use tokio_serde::formats::*;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use crate::{
    counter,
    counter_types::{get_epoc_minutes, CounterMessage, CounterState},
};

struct TimeBucketSpec {
    interval_minutes: u64,
    interval_count: usize,
}

const TIME_BUCKET_COUNT: usize = 5;
// Newest to oldest
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

struct TimeBucket {
    interval_minutes: u64,
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

                updateTimeSeries(
                    &mut *time_series.lock().unwrap(),
                    msg.state,
                    server_epoch_minutes,
                );
            }
        });
    }
}

fn updateTimeSeries(
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
        let (younger, older) = time_series.split_at_mut(i + 1);
        let bucket = younger.last_mut().unwrap();
        // Oldest to earliest bucket
        while bucket.data.back_mut().map_or(false, |last_state| {
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
        return;
    }

    if time_series[0].cutoff_minutes < server_epoch_minutes - counter_state.epoch_minutes {
        // The counter state is too old for the current time series bucket
        add_to_series(&mut time_series[1..], counter_state, server_epoch_minutes);
        return;
    }

    let bucket = &mut time_series[0];
    add_to_bucket(bucket, counter_state);
}

fn add_to_bucket(bucket: &mut TimeBucket, mut counter_state: CounterState) {
    if counter_state.epoch_minutes > bucket.cutoff_minutes {
        panic!("Counter state is too old for the bucket");
    }

    for i in (0..bucket.data.len()).rev() {
        // Handle updates of any age, but generally expect the newest state to be updated
        // i.e. Return after the first iteration
        let interval_state = &mut bucket.data[i];
        if counter_state.epoch_minutes < interval_state.epoch_minutes {
            // New state comes before the current interval
            // Give the new counter state an aligned minute value
            counter_state.epoch_minutes = counter_state.epoch_minutes
                - (counter_state.epoch_minutes % bucket.interval_minutes);
            bucket.data.insert(i, counter_state);
            return;
        } else if counter_state.epoch_minutes + bucket.interval_minutes
            < interval_state.epoch_minutes
        {
            // New state comes falls into the current interval
            interval_state.count += counter_state.count;
            return;
        }
    }
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
