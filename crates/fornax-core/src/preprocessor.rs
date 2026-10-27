use crate::metadata::Levels;
use crate::{DecodedRawActive, Primitive, RawImage};

pub trait IPreprocessor<T>: Sized
where
    T: Primitive,
{
    fn preprocess(&self, decoded: DecodedRawActive<T>, levels: &Levels<T>) -> RawImage<T>;
}
