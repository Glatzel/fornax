use std::path::Path;

use fornax_core::{FornaxError, FornaxPrimitive, IDecoder, IPostProcessor};
use image::{EncodableLayout, Rgb};

use crate::{Libraw, LibrawError, ProcFlag};

impl From<LibrawError> for FornaxError {
    fn from(val: LibrawError) -> Self { FornaxError(val.to_string()) }
}
impl<T> IDecoder<T> for Libraw
where
    T: FornaxPrimitive,
{
    fn decode_file(&self, file: &Path) -> Result<(), FornaxError> {
        self.open_file(file)?;
        self.unpack()?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> Result<(), FornaxError> {
        self.open_buffer(buffer)?;
        self.unpack()?;
        Ok(())
    }
    fn bayer_image(&self) -> Result<fornax_core::BayerImage<T>, FornaxError> {
        Ok(self.get_bayer_image()?)
    }
}
impl<T> IDecoder<T> for &Libraw
where
    T: FornaxPrimitive,
{
    fn decode_file(&self, file: &Path) -> Result<(), FornaxError> {
        self.open_file(file)?;
        self.unpack()?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> Result<(), FornaxError> {
        self.open_buffer(buffer)?;
        self.unpack()?;
        Ok(())
    }
    fn bayer_image(&self) -> Result<fornax_core::BayerImage<T>, FornaxError> {
        Ok(self.get_bayer_image()?)
    }
}

impl<D, O> IPostProcessor<D, u16, O> for Libraw
where
    D: IDecoder<u16>,
    O: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> Result<image::ImageBuffer<Rgb<O>, Vec<O>>, FornaxError> {
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
            params.set_output_params(self.imgdata.clone())?;
        }
        let processed = self.dcraw_process()?.dcraw_make_mem_image()?;
        Ok(self.map_processed_image(&processed)?)
    }
}

impl<D, O> IPostProcessor<D, u16, O> for &Libraw
where
    D: IDecoder<u16>,
    O: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> Result<image::ImageBuffer<Rgb<O>, Vec<O>>, FornaxError> {
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
            params.set_output_params(self.imgdata.clone())?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (**self.imgdata).params });
        let processed = self.dcraw_process()?.dcraw_make_mem_image()?;
        Ok(self.map_processed_image(&processed)?)
    }
}
