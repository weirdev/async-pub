use std::cell::UnsafeCell;
use std::mem::swap;
// switch to tokio::sync::mpsc
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::RwLock;
use std::thread::JoinHandle;

pub struct LogPublisherHandle<'a, T> {
    thread_handle: Option<JoinHandle<()>>,
    common_sender: &'a CommonSender<T>,
}

pub fn initiate_logging<'a, T, F>(
    common_sender: &'a CommonSender<T>,
    mut publisher: F,
) -> LogPublisherHandle<'a, T>
where
    F: FnMut(T) + Send + 'static,
    T: Send + 'static,
{
    let rx = common_sender.set_new_channel();

    let publisher_thread = std::thread::spawn(move || {
        // This thread will run so long as there is a sender alive.
        // Since common sender keeps an instance of the sender,
        // the thread runs until common sender is dropped or
        // set_new_channel is called again.
        for data in rx {
            publisher(data);
        }
    });

    LogPublisherHandle {
        thread_handle: Some(publisher_thread),
        common_sender,
    }
}

impl<'a, T> Drop for LogPublisherHandle<'a, T> {
    fn drop(&mut self) {
        // Close the sender so the publisher thread can exit
        self.common_sender.close_channel();
        // Wait for the publisher thread to exit
        let mut thread_handle: Option<JoinHandle<()>> = None;
        swap(&mut self.thread_handle, &mut thread_handle);
        thread_handle.unwrap().join().unwrap();
    }
}

pub struct CommonSender<T> {
    tx: RwLock<Option<Sender<T>>>,
}

unsafe impl<T> Sync for CommonSender<T> {}

impl<T> CommonSender<T> {
    pub const fn new() -> CommonSender<T> {
        CommonSender {
            tx: RwLock::new(None),
        }
    }

    pub fn send(&self, data: T) -> Result<(), ()> {
        let s = self
            .tx
            .read()
            .unwrap()
            .as_ref()
            .map(|s| s.clone())
            .ok_or(())?;
        s.send(data).map_err(|_| ())
    }

    fn set_new_channel(&self) -> Receiver<T> {
        let (tx, rx) = channel::<T>();

        // Must clone the sender to ensure the same sender not used by multiple threads
        // The clone is dropped, but the original sender must stay alive
        // to keep the channel open
        let mut old_tx_opt = self.tx.write().unwrap();
        let _ = std::mem::replace(&mut *old_tx_opt, Some(tx));

        rx
    }

    fn close_channel(&self) {
        let mut old_tx_opt = self.tx.write().unwrap();
        let _ = std::mem::replace(&mut *old_tx_opt, None);
    }
}

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
