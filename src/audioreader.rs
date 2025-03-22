#[allow(unused_imports)]
pub use crate::errors::*;

use crate::audiocore::{Spec};
use crate::sampleutils::SampleConv;

pub trait AudioReader {
    fn spec(&self) -> &Spec;

    fn iter<T>(&mut self) -> Result<Box<dyn AudioIter<T>>, Box<dyn std::error::Error>> where Self: Sized, T: SampleConv;
}

pub trait AudioIter<T: SampleConv>: Iterator<Item = Vec<T>> {}
