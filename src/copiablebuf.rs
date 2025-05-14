#![allow(dead_code)]
use std::{
    cmp::{min, PartialEq},
    fmt::Debug,
    iter::FromIterator,
    ops::{Index, IndexMut, Deref, DerefMut, Range, RangeFrom, RangeTo, RangeFull},
};

pub trait CopiableItem: Default + Clone + Copy + Debug + Sized + PartialEq {}
impl<T> CopiableItem for T where T: Default + Clone + Copy + Debug + Sized + PartialEq {}

/// * Copiable buffer, a tinier `Vec`, uses a fixed-size array to store a variable number of items.
#[derive(Clone, Copy, Eq)]
pub struct CopiableBuffer<T, const N: usize>
where
    T: CopiableItem,
{
    buf_used: usize,
    buffer: [T; N],
}

/// * The iterator for the copiable buffer
#[derive(Debug)]
pub struct CopiableBufferIter<'a, T, const N: usize>
where
    T: CopiableItem,
{
    iter_index: usize,
    refbuf: &'a CopiableBuffer<T, N>,
}

/// * The mutable iterator for the copiable buffer
#[derive(Debug)]
pub struct CopiableBufferIterMut<'a, T, const N: usize>
where
    T: CopiableItem,
{
    iter_index: usize,
    refbuf: &'a mut CopiableBuffer<T, N>,
}

/// * The iterator with owned data for the copiable buffer
#[derive(Clone, Copy)]
pub struct CopiableBufferIntoIter<T, const N: usize>
where
    T: CopiableItem,
{
    iter_index: usize,
    buf_used: usize,
    buffer: [T; N],
}

impl<T, const N: usize> CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    pub fn new() -> Self {
        Self {
            buf_used: 0,
            buffer: [T::default(); N],
        }
    }

    pub fn len(&self) -> usize {
        self.buf_used
    }

    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.buf_used = new_len;
    }

    pub fn resize(&mut self, new_len: usize, val: T) {
        if new_len > N {
            panic!("The new size {new_len} exceeded `CopiableBuffer<{}, {N}>` capacity", std::any::type_name::<T>());
        }
        if new_len > self.buf_used {
            for i in self.buf_used..new_len {
                unsafe {
                    *self.buffer.get_unchecked_mut(i) = val;
                }
            }
        }
        self.buf_used = new_len;
    }

    pub fn truncate(&mut self, new_len: usize) {
        self.buf_used = min(self.buf_used, new_len);
    }

    pub fn last(&mut self) -> &mut T {
        if self.buf_used == 0 {
            panic!(
                "CopiableBuffer<{}, {N}> is empty.",
                std::any::type_name::<T>()
            );
        }
        unsafe{self.buffer.get_unchecked_mut(self.buf_used - 1)}
    }

    pub fn push(&mut self, data: T) {
        if self.buf_used >= self.buffer.len() {
            panic!(
                "CopiableBuffer<{}, {N}> is full.",
                std::any::type_name::<T>()
            );
        } else {
            unsafe{*self.buffer.get_unchecked_mut(self.buf_used) = data;}
            self.buf_used += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        CopiableBufferIter::<T, N> {
            refbuf: self,
            iter_index: 0,
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        CopiableBufferIterMut::<T, N> {
            refbuf: self,
            iter_index: 0,
        }
    }

    pub fn clear(&mut self) {
        unsafe{self.set_len(0)}
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_array(&self) -> &[T; N] {
        &self.buffer
    }

    pub fn into_array(self) -> [T; N] {
        self.buffer
    }
}

impl<T, const N: usize> Default for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> PartialEq for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    // Required method
    fn eq(&self, other: &Self) -> bool {
        self[..] == other[..]
    }
}

impl<T, const N: usize> Index<usize> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.buf_used {
            panic!("Index out of bounds: {} >= {}", index, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked(index)}
    }
}

impl<T, const N: usize> IndexMut<usize> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn index_mut(&mut self, index: usize) -> &mut T {
        if index >= self.buf_used {
            panic!("Index out of bounds: {} >= {}", index, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked_mut(index)}
    }
}

impl<T, const N: usize> Index<Range<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    fn index(&self, range: Range<usize>) -> &[T] {
        if range.start >= self.buf_used {
            panic!("Index out of bounds: {} >= {}", range.start, self.buf_used);
        }
        if range.end > self.buf_used {
            panic!("Index out of bounds: {} >= {}", range.end, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked(range)}
    }
}

impl<T, const N: usize> IndexMut<Range<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn index_mut<'a>(&mut self, range: Range<usize>) -> &mut [T] {
        if range.start >= self.buf_used {
            panic!("Slice start is out of bounds: {} >= {}", range.start, self.buf_used);
        }
        if range.end > self.buf_used {
            panic!("Slice end is out of bounds: {} >= {}", range.end, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked_mut(range)}
    }
}

impl<T, const N: usize> Index<RangeFrom<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    fn index(&self, range: RangeFrom<usize>) -> &[T] {
        if range.start >= self.buf_used {
            panic!("Slice start is of bounds: {} >= {}", range.start, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked(range)}
    }
}

impl<T, const N: usize> IndexMut<RangeFrom<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn index_mut<'a>(&mut self, range: RangeFrom<usize>) -> &mut [T] {
        if range.start >= self.buf_used {
            panic!("Slice start is of bounds: {} >= {}", range.start, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked_mut(range)}
    }
}

impl<T, const N: usize> Index<RangeTo<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    fn index(&self, range: RangeTo<usize>) -> &[T] {
        if range.end > self.buf_used {
            panic!("Slice end is out of bounds: {} >= {}", range.end, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked(range)}
    }
}

impl<T, const N: usize> IndexMut<RangeTo<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn index_mut<'a>(&mut self, range: RangeTo<usize>) -> &mut [T] {
        if range.end > self.buf_used {
            panic!("Slice end is out of bounds: {} >= {}", range.end, self.buf_used);
        }
        unsafe{self.buffer.get_unchecked_mut(range)}
    }
}

impl<T, const N: usize> Index<RangeFull> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    fn index(&self, _range: RangeFull) -> &[T] {
        unsafe{self.buffer.get_unchecked(..self.buf_used)}
    }
}

impl<T, const N: usize> IndexMut<RangeFull> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn index_mut<'a>(&mut self, _range: RangeFull) -> &mut [T] {
        unsafe{self.buffer.get_unchecked_mut(..self.buf_used)}
    }
}

impl<T, const N: usize> Deref for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe{self.buffer.get_unchecked(..self.buf_used)}
    }
}

impl<T, const N: usize> DerefMut for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe{self.buffer.get_unchecked_mut(..self.buf_used)}
    }
}

impl<T, const N: usize> FromIterator<T> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut ret = Self::new();
        for data in iter {
            ret.push(data);
        }
        ret
    }
}

impl<'a, T, const N: usize> Iterator for CopiableBufferIter<'a, T, N>
where
    T: CopiableItem,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_index < self.refbuf.len() {
            let r = unsafe{self.refbuf.buffer.get_unchecked(self.iter_index)};
            self.iter_index += 1;
            Some(r)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter_index += n;
        self.next()
    }
}

impl<'a, T, const N: usize> Iterator for CopiableBufferIterMut<'a, T, N>
where
    T: CopiableItem,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_index < self.refbuf.len() {
            unsafe {
                let item_ptr = self.refbuf.buffer.as_mut_ptr().add(self.iter_index);
                self.iter_index += 1;
                Some(&mut *item_ptr)
            }
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter_index += n;
        self.next()
    }
}

impl<T, const N: usize> Iterator for CopiableBufferIntoIter<T, N>
where
    T: CopiableItem,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_index < self.buf_used {
            let ret = Some(unsafe{*self.buffer.get_unchecked(self.iter_index)});
            self.iter_index += 1;
            ret
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter_index += n;
        self.next()
    }
}

impl<T, const N: usize> IntoIterator for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Item = T;
    type IntoIter = CopiableBufferIntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            buffer: self.buffer,
            buf_used: self.buf_used,
            iter_index: 0,
        }
    }
}

impl<T, const N: usize> Debug for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct(&format!(
            "CopiableBuffer<{}, {N}>",
            std::any::type_name::<T>()
        ))
        .field("buf_used", &self.buf_used)
        .field("buffer", &&self[..])
        .finish()
    }
}

impl<T, const N: usize> Debug for CopiableBufferIntoIter<T, N>
where
    T: CopiableItem,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct(&format!(
            "CopiableBufferIntoIter<{}, {N}>",
            std::any::type_name::<T>()
        ))
        .field("iter_index", &self.iter_index)
        .field("buf_used", &self.buf_used)
        .field("buffer", &&self.buffer[..self.buf_used])
        .finish()
    }
}
