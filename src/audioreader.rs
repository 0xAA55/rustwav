#[allow(unused_imports)]
pub use crate::errors::*;

use crate::audiocore::{Spec};
use crate::sampleutils::SampleConv;

pub trait AudioReader<T: SampleConv> {
    fn spec(&self) -> &Spec;

    fn iter<T>(&mut self) -> Result<Iterator<Item = Vec<T>>, Box<dyn std::error::Error>> where Self: Sized;
}
