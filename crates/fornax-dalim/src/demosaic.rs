mod linear;
pub use linear::DemosaicLinear;
pub trait IDemosaic {
    fn demosaic(
        bayer_image: &fornax_core::BayerImage,
    ) -> image::ImageBuffer<image::Rgb<u16>, Vec<u16>>;
}
