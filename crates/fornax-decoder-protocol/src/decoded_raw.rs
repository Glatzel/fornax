use fornax_core::{MonoImage, Primitive};
use image::math::Rect;
#[derive(Clone, Debug)]
pub struct DecodedRawFull<T>(MonoImage<T>)
where
    T: Primitive;

impl<T> DecodedRawFull<T>
where
    T: Primitive,
{
    pub const fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn active_area(mut self, active_area: Rect) -> DecodedRawActive<T> {
        self.0.crop_in_place(active_area);
        DecodedRawActive(self.0)
    }
}

#[derive(Clone, Debug)]
pub struct DecodedRawActive<T>(MonoImage<T>)
where
    T: Primitive;
impl<T> DecodedRawActive<T>
where
    T: Primitive,
{
    pub const fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn into_image(self) -> MonoImage<T> { self.0 }
}
