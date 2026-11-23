pub type MonoImage<T> = image::ImageBuffer<image::Luma<T>, Vec<T>>;
pub type UvImage<T> = image::ImageBuffer<image::LumaA<T>, Vec<T>>;
pub type RgbImage<T> = image::ImageBuffer<image::Rgb<T>, Vec<T>>;
pub type RgbaImage<T> = image::ImageBuffer<image::Rgba<T>, Vec<T>>;
pub type MultiLayerImage<T> = Vec<image::ImageBuffer<image::Luma<T>, Vec<T>>>;
