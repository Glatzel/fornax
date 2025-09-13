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
        self.open_file(file).map_err(FornaxError::from)?;
        self.unpack().map_err(FornaxError::from)?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> Result<(), FornaxError> {
        self.open_buffer(buffer).map_err(FornaxError::from)?;
        self.unpack().map_err(FornaxError::from)?;
        Ok(())
    }
    fn bayer_image(&self) -> Result<fornax_core::BayerImage<T>, FornaxError> {
        self.get_bayer_image().map_err(FornaxError::from)
    }
}
impl<T> IDecoder<T> for &Libraw
where
    T: FornaxPrimitive,
{
    fn decode_file(&self, file: &Path) -> Result<(), FornaxError> {
        self.open_file(file).map_err(FornaxError::from)?;
        self.unpack().map_err(FornaxError::from)?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> Result<(), FornaxError> {
        self.open_buffer(buffer).map_err(FornaxError::from)?;
        self.unpack().map_err(FornaxError::from)?;
        Ok(())
    }
    fn bayer_image(&self) -> Result<fornax_core::BayerImage<T>, FornaxError> {
        self.get_bayer_image().map_err(FornaxError::from)
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
        )
        .map_err(FornaxError::from)?;
        self.unpack().map_err(FornaxError::from)?;
        if let Some(params) = &self.params {
            params
                .set_output_params(self.imgdata)
                .map_err(FornaxError::from)?;
        }
        let processed = self
            .dcraw_process()
            .map_err(FornaxError::from)?
            .dcraw_make_mem_image()
            .map_err(FornaxError::from)?;
        self.map_processed_image(&processed)
            .map_err(FornaxError::from)
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
        )
        .map_err(FornaxError::from)?;
        self.unpack().map_err(FornaxError::from)?;
        if let Some(params) = &self.params {
            params
                .set_output_params(self.imgdata)
                .map_err(FornaxError::from)?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (*self.imgdata).params });
        let processed = self
            .dcraw_process()
            .map_err(FornaxError::from)?
            .dcraw_make_mem_image()
            .map_err(FornaxError::from)?;
        self.map_processed_image(&processed)
            .map_err(FornaxError::from)
    }
}
