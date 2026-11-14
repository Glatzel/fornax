use fornax_codec::CodecError;
use fornax_raw_core::metadata::{EncodingMetadata, EncodingSampleFormat};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonCrxHeader {
    cmp1_version: u16,
    mosaic_width: u32,
    mosaic_height: u32,
    tile_width: u32,
    tile_height: u32,
    bit_depth: u8,
    encoding_type: u8,
    transform_levels: u8,
    mdat_header_size: u32,
    median_precision: u8,
}

impl CanonCrxHeader {
    pub const MAX_PIXELS: u64 = 120_000_000;

    pub fn new(
        cmp1_version: u16,
        mosaic_width: u32,
        mosaic_height: u32,
        tile_width: u32,
        tile_height: u32,
        bit_depth: u8,
        encoding_type: u8,
        transform_levels: u8,
        mdat_header_size: u32,
        median_precision: u8,
    ) -> Result<Self, CodecError> {
        let pixels = u64::from(mosaic_width) * u64::from(mosaic_height);
        if mosaic_width == 0 || mosaic_height == 0 || tile_width == 0 || tile_height == 0 {
            return Err(CodecError::InvalidInput(
                "Canon CRX geometry must not be zero".into(),
            ));
        }
        if pixels > Self::MAX_PIXELS {
            return Err(CodecError::InvalidInput(
                "Canon CRX mosaic exceeds pixel cap".into(),
            ));
        }
        if !(8..=16).contains(&bit_depth) || median_precision != bit_depth {
            return Err(CodecError::InvalidInput(
                "Canon CRX lossless precision must be 8..=16 bits".into(),
            ));
        }
        if cmp1_version != 0x0100
            || encoding_type != 0
            || transform_levels != 0
            || mdat_header_size != 0x70
        {
            return Err(CodecError::Unsupported(
                "Canon CRX CMP1 shape is not the validated lossless layout".into(),
            ));
        }
        Ok(Self {
            cmp1_version,
            mosaic_width,
            mosaic_height,
            tile_width,
            tile_height,
            bit_depth,
            encoding_type,
            transform_levels,
            mdat_header_size,
            median_precision,
        })
    }

    pub const fn cmp1_version(&self) -> u16 { self.cmp1_version }

    pub const fn mosaic_width(&self) -> u32 { self.mosaic_width }

    pub const fn mosaic_height(&self) -> u32 { self.mosaic_height }

    pub const fn tile_width(&self) -> u32 { self.tile_width }

    pub const fn tile_height(&self) -> u32 { self.tile_height }

    pub const fn bit_depth(&self) -> u8 { self.bit_depth }

    pub const fn encoding_type(&self) -> u8 { self.encoding_type }

    pub const fn transform_levels(&self) -> u8 { self.transform_levels }

    pub const fn mdat_header_size(&self) -> u32 { self.mdat_header_size }

    pub const fn median_precision(&self) -> u8 { self.median_precision }

    /// Return source encoding metadata for the validated lossless CRX sample.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        EncodingMetadata {
            compression_id: None,
            compression_name: "canon-lossless-crx",
            storage: "bmff-sample",
            bits_per_sample: Some(u16::from(self.bit_depth)),
            sample_format: Some(EncodingSampleFormat::Integer),
            predictor: None,
            rows_per_strip: None,
            tile_width: Some(self.tile_width),
            tile_height: Some(self.tile_height),
        }
    }
}

#[cfg(test)]
mod tests {
    use fornax_raw_core::EncodingSampleFormat;

    use super::*;

    #[test]
    fn encoding_metadata_describes_lossless_crx_layout() {
        let header =
            CanonCrxHeader::new(0x0100, 8_000, 6_000, 512, 384, 14, 0, 0, 0x70, 14).unwrap();

        let encoding = header.encoding_metadata();

        assert_eq!(encoding.compression_id, None);
        assert_eq!(encoding.compression_name, "canon-lossless-crx");
        assert_eq!(encoding.storage, "bmff-sample");
        assert_eq!(encoding.bits_per_sample, Some(14));
        assert_eq!(encoding.sample_format, Some(EncodingSampleFormat::Integer));
        assert_eq!(encoding.tile_width, Some(512));
        assert_eq!(encoding.tile_height, Some(384));
    }
}
