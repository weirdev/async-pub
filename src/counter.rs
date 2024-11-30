// Adapt the generic logger for use as a count publisher

use std::collections::HashMap;
use std::thread;

use futures::prelude::*;
use tokio::net::TcpStream;
use tokio_serde::formats::Json;
use tokio_util::codec::{Framed, FramedWrite, LengthDelimitedCodec};

use crate::counter_types::{get_epoc_minutes, CounterMessage, CounterState, CounterUpdateMessage};
use crate::logger::{Logger, Publisher};

struct CountersStruct(Logger<String>);
static COUNTERS: CountersStruct = CountersStruct(Logger::new::<CounterPublishState>());

struct CounterPublishState {
    counters: HashMap<String, CounterState>,
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
        let message = CounterUpdateMessage { counter, state };

        // TODO: Maintain just one connection (or pool or thread)

        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap()
            .block_on(transport_message(CounterMessage::Update(message)))
            .unwrap();
    }
}

pub async fn get_count_async(counter: String) -> usize {
    // TODO: Super hacky, register the read before we send the request
    let receive = receive_message();

    transport_message(CounterMessage::Read(counter))
        .await
        .unwrap();
    receive.await.unwrap().count
}

async fn get_typed_socket() -> tokio_serde::Framed<
    Framed<TcpStream, LengthDelimitedCodec>,
    CounterState,
    CounterMessage,
    Json<CounterState, CounterMessage>,
> {
    let socket = TcpStream::connect("127.0.0.1:7878").await.unwrap();

    let length_delimited = Framed::new(socket, LengthDelimitedCodec::new());

    tokio_serde::Framed::new(
        length_delimited,
        Json::<CounterState, CounterMessage>::default(),
    )
}

async fn transport_message(message: CounterMessage) -> Result<(), std::io::Error> {
    let mut typed_socket = get_typed_socket().await;

    typed_socket.send(message).await
}

async fn receive_message() -> Result<CounterState, std::io::Error> {
    let mut typed_socket = get_typed_socket().await;

    match typed_socket.try_next().await? {
        Some(counter_state) => Ok(counter_state),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No message received",
        )),
    }
}

impl Publisher<String> for CounterPublishState {
    fn new() -> Self {
        CounterPublishState {
            counters: HashMap::new(),
        }
    }

    fn send(&mut self, counter: String) -> Result<(), ()> {
        // ~1 minute accuracy
        let epoch_minutes = get_epoc_minutes();

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

        // Publish if it has been a minute or the count is a power of 2
        // TODO: Start at the power of two from the last minute or 1/2 of it
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
            // panic!("Some counters not published");
        }
    }
}

pub fn inc_counter(counter: String) {
    COUNTERS.0.send(counter).unwrap();
}

pub fn get_count(counter: String) -> usize {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
        .block_on(get_count_async(counter))
}
