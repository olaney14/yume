use std::{mem::MaybeUninit, ops::{Deref, DerefMut}};

pub struct Slot<T> {
    inner: MaybeUninit<T>,
    valid: bool
}

impl<T> Slot<T> {
    pub fn take<F>(&mut self, cl: F) where F: FnOnce(T) -> T {
        assert!(self.valid, "Slot was empty during take. take() must not be called within its own closure.");

        // read is a shallow copy
        // Safety: validity of value ensured with flag check
        let inner_owned = unsafe { self.inner.as_ptr().read() };
        self.valid = false;

        let new = cl(inner_owned);
        self.inner.write(new);
        self.valid = true;
    }

    pub fn get(&self) -> &T {
        assert!(self.valid);
        unsafe { &*self.inner.as_ptr() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        assert!(self.valid);
        unsafe { &mut *self.inner.as_mut_ptr() }
    }
}

impl<T> Deref for Slot<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for Slot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T> Drop for Slot<T> {
    fn drop(&mut self) {
        assert!(self.valid);
        unsafe { self.inner.assume_init_drop(); }
    }
}

// fn sabs(x: f32, lambda: f32) -> f32 {
//     x.abs() + (0.0f32.max(lambda - x.abs())).powi(2) / (2.0 * lambda)
// }

pub fn smax(x: f32, y: f32, lambda: f32) -> f32 {
    x.max(y) + 0.0f32.max(lambda - (x - y).abs()).powi(2) / (4.0 * lambda)
}