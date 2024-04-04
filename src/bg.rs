use std::{
    cell::UnsafeCell, ops::Deref, sync::{LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard}
};

// pub trait Readable<'a, T> {
//     fn read(&self) -> &'a T;
// }

// pub trait Writable<'a, T> {
//     fn write(&self) -> &'a mut T;
// }

// pub trait RW<'a, R, S: Readable<'a, R> + ?Sized, W, X: Writable<'a, W> + ?Sized> {
//     fn read(&'a self) -> S;
//     fn write(&'a self) -> X;
// }

// pub trait RWReplace<'a, R, S: Readable<'a, R> + ?Sized, W: 'a, X: Writable<'a, W>>:
//     RW<'a, R, S, W, X>
// {
//     fn replace(&'a self, data: W) {
//         let _ = std::mem::replace(self.write().write(), data);
//     }
// }

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

// impl<'a, T> Readable<'a, T> for &'a T {
//     fn read(&self) -> &'a T {
//         self
//     }
// }

// impl<'a, T> Writable<'a, T> for &'a UnsafeSyncCell<T> {
//     fn write(&self) -> &'a mut T {
//         unsafe { &mut *self.data.get() }
//     }
// }

// impl<'a, T> RW<'a, T, &'a T, T, &'a UnsafeSyncCell<T>> for UnsafeSyncCell<T> {
//     fn read(&'a self) -> &'a T {
//         unsafe { &*self.data.get() }
//     }

//     fn write(&'a self) -> &'a UnsafeSyncCell<T> {
//         self
//     }
// }

// impl<'a, T> RWReplace<'a, T, &'a T, T, &'a UnsafeSyncCell<T>> for UnsafeSyncCell<T> {}

// pub trait RAppend<'a, R: ?Sized, A> {
//     fn read(&self) -> &R;
//     fn append(&'a self, data: A);
// }

// impl<'a, T> RAppend<'a, [T], T> for UnsafeSyncCell<Vec<T>> {
//     fn read(&self) -> &[T] {
//         RW::read(self)
//     }

//     fn append(&'a self, data: T) {
//         Writable::write(&RW::write(self)).push(data);
//     }
// }

// // TODO: Maybe use evmap
// // Actually channels could be better std::sync::mpsc



// impl<'a> Readable<'a, Vec<u8>> for RwLockReadGuard<'a, Vec<u8>> {
//     fn read(&self) -> &'a Vec<u8> {
//         self.deref()
//     }
// }

// impl<'a> Writable<'a, Vec<u8>> for LockResult<RwLockWriteGuard<'a, Vec<u8>>> {
//     fn write(&self) -> &'a mut Vec<u8> {
//         let s  = self.as_ref().unwrap();
//         &mut *s
//     }
// }

// impl <'a> RW<'a, Vec<u8>, LockResult<RwLockReadGuard<'a, Vec<u8>>>, Vec<u8>, LockResult<RwLockWriteGuard<'a, Vec<u8>>>> for RwLock<Vec<u8>> {
//     fn read(&self) -> LockResult<RwLockReadGuard<'a, Vec<u8>>> {
//         self.read()
//     }

//     fn write(&self) -> LockResult<RwLockWriteGuard<'a, Vec<u8>>> {
//         self.write()
//     }
// }
