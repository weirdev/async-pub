use std::cell::UnsafeCell;

pub trait RW<R, W> {
    fn read(&self) -> &R;
    fn write(&self, data: W);
}

pub struct UnsafeSyncCell<T> {
    pub data: UnsafeCell<T>
}

unsafe impl Sync for UnsafeSyncCell<String> {}

impl<T> RW<T, T> for UnsafeSyncCell<T> {
    fn read(&self) -> &T {
        unsafe { &*self.data.get() }
    }

    fn write(&self, data: T) {
        unsafe { *self.data.get() = data; }
    }
}

pub trait RAppend<R, A> {
    fn read(&self) -> &R;
    fn append(&mut self, data: A);
}

pub struct BGData<T, R, W> where T: RW<R, W> {
    data: T,
    _r: std::marker::PhantomData<R>,
    _w: std::marker::PhantomData<W>
}

impl<T, R, W> BGData<T, R, W> where T: RW<R, W> {
    pub const fn new(data: T) -> BGData<T, R, W> {
        BGData {
            data: data,
            _r: std::marker::PhantomData,
            _w: std::marker::PhantomData
        }
    }

    pub fn read(&self) -> &R {
        self.data.read()
    }

    pub fn write(&self, data: W) {
        self.data.write(data);
    }
}