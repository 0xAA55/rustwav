
#[macro_export]
macro_rules! force_borrow {
	($wanted:expr, $ty:ty) => {
		{
			let writer_raw_ptr = &mut $wanted as *mut $ty;
			unsafe { &mut *writer_raw_ptr }
		}
	}
}
pub use crate::force_borrow;
