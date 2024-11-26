use core::time;
use std::sync::{Arc, Mutex};

use futures::prelude::*;
use ringbuf::{traits::*, HeapRb};
use tokio::net::TcpListener;
use tokio_serde::formats::*;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use crate::counter_types::{get_epoc_minutes, CounterMessage};

struct TimeBucketSpec {
    interval_minutes: usize,
    interval_count: usize,
}

const TIME_BUCKET_COUNT: usize = 5;
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
    interval_minutes: usize,
    data: HeapRb<CounterMessage>,
}

#[tokio::main]
pub async fn run_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;

    let time_series = <TimeSeries>::try_from(
        TIME_BUCKET_SPECS
            .iter()
            .map(|spec| TimeBucket {
                interval_minutes: spec.interval_minutes,
                data: HeapRb::<CounterMessage>::new(spec.interval_count),
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| panic!("Failed to create time series buckets"));
    let time_series = Arc::new(Mutex::new(time_series));

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();

        let time_series = time_series.clone();

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

                let mut ts = time_series.lock().unwrap();
                updateTimeSeries(&mut *ts, msg, server_epoch_minutes);
            }
        });
    }
}

fn updateTimeSeries(time_series: &mut TimeSeries, counter_message: CounterMessage, server_epoch_minutes: u64) {}
