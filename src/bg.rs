use std::cell::UnsafeCell;
use std::mem::swap;
// switch to tokio::sync::mpsc
use std::sync::mpsc::{channel, Sender};
use std::sync::RwLock;
use std::thread::JoinHandle;

use once_cell::sync::Lazy;

pub struct Logger<T> {
    // Lazy: Allows doing the complex (non const) initialization of the state
    // RwLock: Allows multiple read threads to publish simultaneously and a single write thread to close the logger
    // Option: Allows the state to be cleared when the logger is closed
    state: Lazy<RwLock<Option<LoggerState<T>>>>,
}

impl<T> Logger<T> {
    pub const fn new<P>() -> Logger<T>
    where
        P: Publisher<T>,
        T: Send + 'static,
    {
        Self {
            state: Lazy::new(move || {
                let (tx, rx) = channel::<T>();

                let publisher_thread = std::thread::spawn(move || {
                    let mut publisher = P::new();
                    // This thread will run so long as there is a sender alive.
                    // Since common sender keeps an instance of the sender,
                    // the thread runs until common sender is dropped or
                    // set_new_channel is called again.
                    for data in rx {
                        let _ = publisher.send(data);
                    }
                });

                RwLock::new(Some(LoggerState {
                    publisher_handle: Some(publisher_thread),
                    tx: Some(tx),
                }))
            }),
        }
    }

    pub fn send(&self, data: T) -> Result<(), ()> {
        // Must clone the sender to ensure the same sender not used by multiple threads
        // The clone is dropped, but the original sender must stay alive
        // to keep the channel open
        let s = self
            .state
            .read()
            .unwrap()
            .as_ref()
            .map(|state| state.tx.as_ref().unwrap().clone())
            .ok_or(())?;
        s.send(data).map_err(|_| ())
    }

    pub fn close(&self) {
        let mut state = self.state.write().unwrap();
        let mut cleared_state: Option<LoggerState<T>> = None;
        swap(&mut *state, &mut cleared_state);
    }
}

pub trait Publisher<T> {
    fn new() -> Self;
    fn send(&mut self, data: T) -> Result<(), ()>;
}

struct LoggerState<T> {
    // Store state fields as options so they can be safely dropped manually
    // The thread handle is stored so it can be joined when the logger is dropped
    publisher_handle: Option<JoinHandle<()>>,
    tx: Option<Sender<T>>,
}

impl<T> Drop for LoggerState<T> {
    fn drop(&mut self) {
        // Close the sender so the publisher thread can exit
        {
            let mut cleared_tx: Option<Sender<T>> = None;
            swap(&mut self.tx, &mut cleared_tx);
        }
        // Wait for the publisher thread to exit
        let mut thread_handle: Option<JoinHandle<()>> = None;
        swap(&mut self.publisher_handle, &mut thread_handle);
        thread_handle.unwrap().join().unwrap();
    }
}

unsafe impl<T> Sync for Logger<T> {}

pub struct UnsafeSyncCell<T> {
    pub data: UnsafeCell<T>,
}

unsafe impl<T> Sync for UnsafeSyncCell<T> {}

impl<T> UnsafeSyncCell<T> {
    pub const fn new(data: T) -> UnsafeSyncCell<T> {
        UnsafeSyncCell {
            data: UnsafeCell::new(data),
        }
    }
}
