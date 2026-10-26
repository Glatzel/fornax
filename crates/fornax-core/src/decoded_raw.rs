use std::marker::PhantomData;

use image::math::Rect;

use crate::{MonoImage, Primitive};

#[derive(Clone, Copy, Debug)]
pub struct Full;
#[derive(Clone, Copy, Debug)]
pub struct Active;

#[derive(Clone, Debug)]
pub struct DecodedRaw<T: Primitive, S>(MonoImage<T>, PhantomData<S>);

pub type DecodedRawFull<T> = DecodedRaw<T, Full>;
pub type DecodedRawActive<T> = DecodedRaw<T, Active>;

impl<T: Primitive, S> DecodedRaw<T, S> {
    pub const fn new(image: MonoImage<T>) -> Self { Self(image, PhantomData) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn into_image(self) -> MonoImage<T> { self.0 }
}

impl<T: Primitive> DecodedRawFull<T> {
    pub fn active_area(mut self, active_area: Rect) -> DecodedRawActive<T> {
        self.0.crop_in_place(active_area);
        DecodedRaw::new(self.0)
    }
}
