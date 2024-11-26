use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::prelude::*;
use tokio::net::TcpListener;
use tokio_serde::formats::*;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use crate::counter_types::{get_epoc_minutes, CounterMessage};

#[tokio::main]
pub async fn run_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;

    let messages = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();

        let epoch_minutes = get_epoc_minutes();

        let messages = messages.clone();

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
                msg.state.iter().for_each(|counter_state| {
                    println!(
                        "{} [{}]: {}",
                        msg.counter, counter_state.epoch_minutes, counter_state.count
                    );
                });

                messages.lock().unwrap().insert(epoch_minutes, msg);
            }
        });
    }
}
