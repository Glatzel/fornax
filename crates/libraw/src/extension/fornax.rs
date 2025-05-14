use std::path::Path;

use fornax_core::{FornaxPrimitive, IDecoder, IPostProcessor};
use image::{EncodableLayout, Rgb};

use crate::{Libraw, ProcFlag};

// region:fornax
impl<T> IDecoder<T> for Libraw
where
    T: FornaxPrimitive,
{
    fn decode_file(&self, file: &Path) -> miette::Result<()> {
        self.open_file(file)?;
        self.unpack()?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()> {
        self.open_buffer(buffer)?;
        self.unpack()?;
        Ok(())
    }
    fn bayer_image(&self) -> miette::Result<fornax_core::BayerImage<T>> { self.get_bayer_image() }
}
impl<T> IDecoder<T> for &Libraw
where
    T: FornaxPrimitive,
{
    fn decode_file(&self, file: &Path) -> miette::Result<()> {
        self.open_file(file)?;
        self.unpack()?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()> {
        self.open_buffer(buffer)?;
        self.unpack()?;
        Ok(())
    }
    fn bayer_image(&self) -> miette::Result<fornax_core::BayerImage<T>> { self.get_bayer_image() }
}

impl<D, O> IPostProcessor<D, u16, O> for Libraw
where
    D: IDecoder<u16>,
    O: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<image::ImageBuffer<Rgb<O>, Vec<O>>> {
        let bayer = decoder.bayer_image()?;
        self.open_bayer(
            bayer.mosaic().as_bytes(),
            bayer.mosaic().width() as u16,
            bayer.mosaic().height() as u16,
            0,
            0,
            0,
            0,
            ProcFlag::BigEndianData,
            bayer.pattern(),
            0,
            0,
            0,
        )?;
        self.unpack()?;
        if let Some(params) = &self.params {
            params.set_output_params(self.imgdata)?;
        }
        let processed = self.dcraw_process()?.dcraw_make_mem_image()?;
        self.map_processed_image(&processed)
    }
}

impl<D, O> IPostProcessor<D, u16, O> for &Libraw
where
    D: IDecoder<u16>,
    O: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<image::ImageBuffer<Rgb<O>, Vec<O>>> {
        let bayer = decoder.bayer_image()?;
        self.open_bayer(
            bayer.mosaic().as_bytes(),
            bayer.mosaic().width() as u16,
            bayer.mosaic().height() as u16,
            0,
            0,
            0,
            0,
            ProcFlag::BigEndianData,
            bayer.pattern(),
            0,
            0,
            0,
        )?;
        self.unpack()?;
        if let Some(params) = &self.params {
            params.set_output_params(self.imgdata)?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (*self.imgdata).params });
        let processed = self.dcraw_process()?.dcraw_make_mem_image()?;
        self.map_processed_image(&processed)
    }
}
