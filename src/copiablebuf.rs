#![allow(dead_code)]
use std::{
    fmt::Debug,
    iter::FromIterator,
    ops::{Index, IndexMut, Deref, DerefMut, Range, RangeFrom, RangeTo},
};

pub trait CopiableItem: Default + Clone + Copy + Debug + Sized {}
impl<T> CopiableItem for T where T: Default + Clone + Copy + Debug + Sized {}

#[derive(Clone, Copy)]
pub struct CopiableBuffer<T, const N: usize>
where
    T: CopiableItem,
{
    buffer: [T; N],
    buf_used: usize,
}

#[derive(Debug)]
pub struct CopiableBufferIter<'a, T, const N: usize>
where
    T: CopiableItem,
{
    refbuf: &'a CopiableBuffer<T, N>,
    iter_index: usize,
}

#[derive(Debug)]
pub struct CopiableBufferIterMut<'a, T, const N: usize>
where
    T: CopiableItem,
{
    refbuf: &'a mut CopiableBuffer<T, N>,
    iter_index: usize,
}

#[derive(Clone, Copy)]
pub struct CopiableBufferIntoIter<T, const N: usize>
where
    T: CopiableItem,
{
    buffer: [T; N],
    buf_used: usize,
    iter_index: usize,
}

impl<T, const N: usize> CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); N],
            buf_used: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.buf_used
    }

    pub fn set_len(&mut self, new_len: usize) {
        self.buf_used = new_len;
    }

    #[track_caller]
    pub fn last(&mut self) -> &mut T {
        if self.buf_used == 0 {
            panic!(
                "CopiableBuffer<{}, {N}> is empty.",
                std::any::type_name::<T>()
            );
        }
        &mut self.buffer[self.buf_used - 1]
    }

    #[track_caller]
    pub fn push(&mut self, data: T) {
        if self.buf_used >= self.buffer.len() {
            panic!(
                "CopiableBuffer<{}, {N}> is full.",
                std::any::type_name::<T>()
            );
        } else {
            self.buffer[self.buf_used] = data;
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
        self.set_len(0)
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

impl<T, const N: usize> Index<usize> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = T;

    #[track_caller]
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.buf_used {
            panic!("Index out of bounds: {} >= {}", index, self.buf_used);
        }
        &self.buffer[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    #[track_caller]
    fn index_mut<'a>(&mut self, index: usize) -> &mut T {
        if index >= self.buf_used {
            panic!("Index out of bounds: {} >= {}", index, self.buf_used);
        }
        &mut self.buffer[index]
    }
}

impl<T, const N: usize> Index<Range<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    #[track_caller]
    fn index(&self, range: Range<usize>) -> &[T] {
        if range.start >= self.buf_used {
            panic!("Index out of bounds: {} >= {}", range.start, self.buf_used);
        }
        if range.end > self.buf_used {
            panic!("Index out of bounds: {} >= {}", range.end, self.buf_used);
        }
        &self.buffer[range]
    }
}

impl<T, const N: usize> IndexMut<Range<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    #[track_caller]
    fn index_mut<'a>(&mut self, range: Range<usize>) -> &mut [T] {
        if range.start >= self.buf_used {
            panic!("Slice start is out of bounds: {} >= {}", range.start, self.buf_used);
        }
        if range.end > self.buf_used {
            panic!("Slice end is out of bounds: {} >= {}", range.end, self.buf_used);
        }
        &mut self.buffer[range]
    }
}

impl<T, const N: usize> Index<RangeFrom<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    #[track_caller]
    fn index(&self, range: RangeFrom<usize>) -> &[T] {
        if range.start >= self.buf_used {
            panic!("Slice start is of bounds: {} >= {}", range.start, self.buf_used);
        }
        &self.buffer[range]
    }
}

impl<T, const N: usize> IndexMut<RangeFrom<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    #[track_caller]
    fn index_mut<'a>(&mut self, range: RangeFrom<usize>) -> &mut [T] {
        if range.start >= self.buf_used {
            panic!("Slice start is of bounds: {} >= {}", range.start, self.buf_used);
        }
        &mut self.buffer[range]
    }
}

impl<T, const N: usize> Index<RangeTo<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Output = [T];

    #[track_caller]
    fn index(&self, range: RangeTo<usize>) -> &[T] {
        if range.end > self.buf_used {
            panic!("Slice end is out of bounds: {} >= {}", range.end, self.buf_used);
        }
        &self.buffer[range]
    }
}

impl<T, const N: usize> IndexMut<RangeTo<usize>> for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    #[track_caller]
    fn index_mut<'a>(&mut self, range: RangeTo<usize>) -> &mut [T] {
        if range.end > self.buf_used {
            panic!("Slice end is out of bounds: {} >= {}", range.end, self.buf_used);
        }
        &mut self.buffer[range]
    }
}

impl<T, const N: usize> Deref for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    type Target = [T];

    #[track_caller]
    fn deref(&self) -> &Self::Target {
        &self.buffer[0..self.buf_used]
    }
}

impl<T, const N: usize> DerefMut for CopiableBuffer<T, N>
where
    T: CopiableItem,
{
    #[track_caller]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer[0..self.buf_used]
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
            let r = &self.refbuf.buffer[self.iter_index];
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
            let ret = Some(self.buffer[self.iter_index]);
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
        .field("buffer", &&self.buffer[..self.buf_used])
        .field("buf_used", &self.buf_used)
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
        .field("buffer", &&self.buffer[..self.buf_used])
        .field("buf_used", &self.buf_used)
        .field("iter_index", &self.iter_index)
        .finish()
    }
}
