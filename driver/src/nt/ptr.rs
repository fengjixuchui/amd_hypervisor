use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

/// Non-null pointer type.
///
/// This is just a wrapper around `NonNull` with implementations for `Deref` and `DerefMut`. This
/// also has some additional helper functions with the most common use cases to make life a little
/// easier.
///
#[repr(transparent)]
pub struct Pointer<T>(NonNull<T>);

impl<T> Pointer<T> {
    pub fn new(ptr: *mut T) -> Option<Self> {
        Some(Self(NonNull::new(ptr)?))
    }

    pub const fn inner(&self) -> NonNull<T> {
        self.0
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<T> Deref for Pointer<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> DerefMut for Pointer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}
