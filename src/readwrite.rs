#![allow(dead_code)]

pub use std::{io::{self, Read, Write, Seek, SeekFrom}, fmt::Debug};

pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug{}

pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug{}

// 这个宏的作用是：首先你有一个 Arc<Mutex<T>>，你想要使用里面的 &mut T
// 首先 Mutex<T> 这里 lock() 释放出一个 MutexGuard，存入 $guard_name
// 然后调用 MutexGuard 的 deref_mut() 放出 &mut T 你就可以操作其中的 T 了
// MutexGuard 必须活着。一旦它被销毁了，T 就不能用了。
// 这个宏会在当前 scope 里面声明两个变量，一个 $guard_name 和一个 $inner_name
// 这个 $inner_name 就是你的 &mut T。
#[macro_export]
macro_rules! peel_arc_mutex {
    ($arc_mutex_t:expr, $inner_name:ident, $guard_name:ident) => {
        let arc_mutex_t = $arc_mutex_t.clone();
        let mut $guard_name = arc_mutex_t.lock().unwrap();
        let $inner_name = $guard_name.deref_mut();
    };
}

pub use crate::peel_arc_mutex;
