use std::cell::UnsafeCell;

pub trait RW<R: ?Sized, W: ?Sized> {
    fn read(&self) -> &R;
    fn write(&self) -> &mut W;
}

pub trait RWReplace<R: ?Sized, W>: RW<R, W> {
    fn replace(&self, data: W) {
        let _ = std::mem::replace(self.write(), data);
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

impl<T> RW<T, T> for UnsafeSyncCell<T> {
    fn read(&self) -> &T {
        unsafe { &*self.data.get() }
    }

    fn write(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T> RWReplace<T, T> for UnsafeSyncCell<T> {}

pub trait RAppend<R: ?Sized, A> {
    fn read(&self) -> &R;
    fn append(&self, data: A);
}

impl<C, T> RAppend<[T], T> for C
where
    C: RW<Vec<T>, Vec<T>>,
{
    fn read(&self) -> &[T] {
        self.read()
    }

    fn append(&self, data: T) {
        self.write().push(data);
    }
}
