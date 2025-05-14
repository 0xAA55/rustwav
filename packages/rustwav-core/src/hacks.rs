#![allow(dead_code)]
#![allow(unused_imports)]

/// * Not for you to use. Force borrow a thing while cheating the compiler.
#[doc(hidden)]
#[macro_export]
macro_rules! force_borrow {
    ($wanted:expr, $ty:ty) => {{
        let writer_raw_ptr = &$wanted as *const $ty;
        unsafe { &*writer_raw_ptr }
    }};
}

/// * Not for you to use. Force borrow a thing as mutable while cheating the compiler.
#[doc(hidden)]
#[macro_export]
macro_rules! force_borrow_mut {
    ($wanted:expr, $ty:ty) => {{
        let writer_raw_ptr = &mut $wanted as *mut $ty;
        unsafe { &mut *writer_raw_ptr }
    }};
}

pub use crate::force_borrow;
pub use crate::force_borrow_mut;
