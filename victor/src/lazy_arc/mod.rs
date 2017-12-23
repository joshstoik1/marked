// Copyright 2017 Simon Sapin
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option.

//! Similar to https://crates.io/crates/lazy_static but:
//!
//! * The static value can be “deinitialized” (dropped).
//!   `Arc` is used to do so safely without invalidating existing references.
//! * Initialization can return an error (for example if it involves parsing).
//!
//! # Example
//!
//! ```rust
//! static FOO: LazyArc<Foo> = LazyArc::INIT;
//!
//! let foo = FOO.get_or_create(|| Ok(Arc::new(include_str!("something").parse()?))?;
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop};
use self::raw_mutex::RawMutex;

mod raw_mutex;

pub struct LazyArc<T: Send + Sync> {
    poltergeist: PhantomData<Arc<T>>,
    mutex: RawMutex,
    ptr: AtomicUsize,
}

impl<T: Send + Sync> LazyArc<T> {
    pub const INIT: Self = LazyArc {
        poltergeist: PhantomData,
        mutex: RawMutex::INIT,
        ptr: ATOMIC_USIZE_INIT,
    };

    // FIXME: figure out minimal Ordering for atomic operations

    /// Return a new `Arc` reference to the singleton `T` object.
    ///
    /// If this singleton was not already initialized,
    /// try to call the closure now (this may return an error) to initialize it.
    ///
    /// Calling this reapeatedly will only initialize once (until `.drop()` is called).
    pub fn get_or_create<F, E>(&self, create: F) -> Result<Arc<T>, E>
        where F: FnOnce() -> Result<Arc<T>, E>
    {
        macro_rules! try_load {
            () => {
                let ptr = self.ptr.load(Ordering::SeqCst);
                if ptr != 0 {
                    // Already initialized

                    // We want to create a new strong reference (with `clone()`)
                    // but not drop the existing one.
                    // `Arc::from_raw` normally takes ownership of a strong reference,
                    // so use `ManuallyDrop` to skip running that destructor.
                    let ptr = ptr as *const T;
                    let careful_dont_drop_it = ManuallyDrop::new(unsafe { Arc::from_raw(ptr) });
                    return Ok(Arc::clone(&*careful_dont_drop_it))
                }
            }
        }

        // First try to obtain an Arc from the atomic pointer without taking the mutex
        try_load!();

        // Synchronize initialization
        struct RawMutexGuard<'a>(&'a RawMutex);
        impl<'a> Drop for RawMutexGuard<'a> {
            fn drop(&mut self) {
                self.0.unlock()
            }
        }

        self.mutex.lock();
        let _guard = RawMutexGuard(&self.mutex);

        // Try again in case some other thread raced us while we were taking the mutex
        try_load!();

        // Now we’ve observed the atomic pointer uninitialized after taking the mutex:
        // we’re definitely first

        let data = create()?;
        let new_ptr = Arc::into_raw(data.clone()) as usize;
        self.ptr.store(new_ptr, Ordering::SeqCst);
        Ok(data)
    }

    /// Deinitialize this singleton, dropping the internal `Arc` reference.
    ///
    /// Calling `.get()` again afterwards will create a new `T` object.
    ///
    /// The previous `T` object may continue to live as long
    /// as other `Arc` references to it exist.
    pub fn drop(&self) {
        let ptr = self.ptr.swap(0, Ordering::SeqCst);
        if ptr != 0 {
            unsafe {
                mem::drop(Arc::from_raw(ptr as *const T))
            }
        }
    }
}
