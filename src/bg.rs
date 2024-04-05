use std::cell::UnsafeCell;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::RwLock;

pub struct CommonSender<T> {
    tx: RwLock<Option<Sender<T>>>,
}

unsafe impl<T> Sync for CommonSender<T> {}

impl<T> CommonSender<T> {
    pub const fn new() -> CommonSender<T> {
        CommonSender { tx: RwLock::new(None) }
    }

    pub fn send(&self, data: T) -> Result<(), ()> {
        let s = self.tx.read().unwrap().as_ref().map(|s| s.clone()).ok_or(())?;
        s.send(data).map_err(|_| ())
    }

    pub fn set_new_channel(&self) -> Receiver<T> {
        let (tx, rx) = channel::<T>();

        // Must clone the sender to ensure the same sender not used by multiple threads
        // The clone is dropped, but the original sender must stay alive
        // to keep the channel open
        let mut old_tx_opt = self.tx.write().unwrap();
        let _ = std::mem::replace(&mut *old_tx_opt, Some(tx));

        rx
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
