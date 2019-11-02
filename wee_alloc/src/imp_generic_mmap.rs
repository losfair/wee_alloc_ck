use super::AllocErr;
use const_init::ConstInit;
use core::ptr;
#[cfg(feature = "extra_assertions")]
use core::cell::Cell;
use memory_units::{Bytes, Pages};
use spin::Mutex;

/// User-provided mmap implementation.
pub static mut MMAP_IMPL: Option<fn (bytes: usize) -> Option<ptr::NonNull<u8>>> = None;

pub(crate) fn alloc_pages(pages: Pages) -> Result<ptr::NonNull<u8>, AllocErr> {
    unsafe {
        let bytes: Bytes = pages.into();
        if let Some(mmap) = MMAP_IMPL {
            let addr = mmap(bytes.0);
            if let Some(addr) = addr {
                Ok(addr)
            } else {
                Err(AllocErr)
            }
        } else {
            Err(AllocErr)
        }
    }
}

pub(crate) struct Exclusive<T> {
    inner: Mutex<T>,

    #[cfg(feature = "extra_assertions")]
    in_use: Cell<bool>,
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        inner: Mutex::new(T::INIT),

        #[cfg(feature = "extra_assertions")]
        in_use: Cell::new(false),
    };
}

extra_only! {
    fn assert_not_in_use<T>(excl: &Exclusive<T>) {
        assert!(!excl.in_use.get(), "`Exclusive<T>` is not re-entrant");
    }
}

extra_only! {
    fn set_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(true);
    }
}

extra_only! {
    fn set_not_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(false);
    }
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// It is the callers' responsibility to ensure that `f` does not re-enter
    /// this method for this `Exclusive` instance.
    //
    // XXX: If we don't mark this function inline, then it won't be, and the
    // code size also blows up by about 200 bytes.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        for<'x> F: FnOnce(&'x mut T) -> U,
    {
        let mut guard = self.inner.lock();
        assert_not_in_use(self);
        set_in_use(self);
        let result = f(&mut guard);
        set_not_in_use(self);
        result
    }
}
