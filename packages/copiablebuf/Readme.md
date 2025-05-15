# Copiable buffer

The copiable buffer is a tinier `Vec`, which uses a fixed-size array to store a variable number of items.

## Overview

### Prototypes
```rust
pub trait CopiableItem: Default + Clone + Copy + Debug + Sized + PartialEq {}
impl<T> CopiableItem for T where T: Default + Clone + Copy + Debug + Sized + PartialEq {}

#[derive(Clone, Copy, Eq)]
pub struct CopiableBuffer<T, const N: usize>
where
    T: CopiableItem,
{
    buf_used: usize,
    buffer: [T; N],
}
```

### Usage

```
let buf = CopiableBuffer::<i32, 64>::new();
```

Then you can use the `buf` just like using a fixed capacity `Vec`.

## Stack memory usage

If you are using this directly in your `struct`, beware if you then just use your `struct` in your `main()` function or test functions, this thing stores data **on the stack**.

Using too much of `CopiableBuffer` on the stack will cause stack overflow even if you are not using recursive calls in your program.

This is a convenient tool for you to create data structures with a clear max size and less memory allocation (Rust only needs to do one memory allocation to your `struct`, then all of the `CopiableBuffer` in your struct were allocated with a clear capacity). If your `struct` is on the heap, all of the `CopiableBuffer` are on the heap too, thus you don't need to worry about stack overflow.
