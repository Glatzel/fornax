use std::io::{Seek, SeekFrom, Write};

use crate::{Tag, TiffError, Value};

/// A value convertible into a TIFF field.
pub trait IntoValue {
    /// Converts this value into a TIFF field.
    fn into_tiff_value(self) -> Value;
}

impl IntoValue for u8 {
    fn into_tiff_value(self) -> Value { Value::Byte(self) }
}

impl IntoValue for u16 {
    fn into_tiff_value(self) -> Value { Value::Short(self) }
}

impl IntoValue for u32 {
    fn into_tiff_value(self) -> Value { Value::Unsigned(self) }
}

impl IntoValue for f32 {
    fn into_tiff_value(self) -> Value { Value::Float(self) }
}

impl IntoValue for &str {
    fn into_tiff_value(self) -> Value { Value::Ascii(self.to_owned()) }
}

impl IntoValue for [u8; 4] {
    fn into_tiff_value(self) -> Value { Value::List(self.into_iter().map(Value::Byte).collect()) }
}

impl IntoValue for &[u16] {
    fn into_tiff_value(self) -> Value {
        Value::List(self.iter().copied().map(Value::Short).collect())
    }
}

impl IntoValue for &[u32] {
    fn into_tiff_value(self) -> Value {
        Value::List(self.iter().copied().map(Value::Unsigned).collect())
    }
}

impl IntoValue for Value {
    fn into_tiff_value(self) -> Value { self }
}

/// A classic little-endian TIFF writer.
pub struct TiffWriter<W> {
    writer: W,
}

impl<W> TiffWriter<W>
where
    W: Write + Seek,
{
    /// Creates a classic TIFF writer with a reserved header.
    pub fn new(mut writer: W) -> Result<Self, TiffError> {
        writer
            .write_all(&[0; 8])
            .map_err(|error| TiffError::new(format!("writing TIFF header failed: {error}")))?;
        Ok(Self { writer })
    }

    /// Appends data and returns its absolute file offset.
    pub fn write_data(&mut self, data: &[u8]) -> Result<u64, TiffError> {
        let offset = self
            .writer
            .stream_position()
            .map_err(|error| TiffError::new(format!("reading TIFF position failed: {error}")))?;
        self.writer
            .write_all(data)
            .map_err(|error| TiffError::new(format!("writing TIFF data failed: {error}")))?;
        Ok(offset)
    }

    /// Writes the root image directory and finalizes the header.
    pub fn finish<I>(&mut self, entries: I) -> Result<(), TiffError>
    where
        I: IntoIterator<Item = (Tag, Value)>,
    {
        let mut encoded = entries
            .into_iter()
            .map(|(tag, value)| EncodedEntry::new(tag, value))
            .collect::<Result<Vec<_>, _>>()?;
        encoded.sort_unstable_by_key(|entry| entry.tag);

        for entry in &mut encoded {
            if entry.bytes.len() <= 4 {
                continue;
            }
            entry.offset = Some(self.write_data(&entry.bytes)?);
        }

        let ifd_offset = self.write_data(&[])?;
        let count = u16::try_from(encoded.len())
            .map_err(|_| TiffError::new("TIFF directory has more than 65535 entries"))?;
        self.writer
            .write_all(&count.to_le_bytes())
            .map_err(write_error)?;
        for entry in &encoded {
            self.writer
                .write_all(&entry.tag.0.to_le_bytes())
                .map_err(write_error)?;
            self.writer
                .write_all(&entry.field_type.to_le_bytes())
                .map_err(write_error)?;
            self.writer
                .write_all(&entry.count.to_le_bytes())
                .map_err(write_error)?;
            if let Some(offset) = entry.offset {
                let offset = u32::try_from(offset)
                    .map_err(|_| TiffError::new("classic TIFF value offset exceeds u32"))?;
                self.writer
                    .write_all(&offset.to_le_bytes())
                    .map_err(write_error)?;
            } else {
                self.writer.write_all(&entry.bytes).map_err(write_error)?;
                for _ in entry.bytes.len()..4 {
                    self.writer.write_all(&[0]).map_err(write_error)?;
                }
            }
        }
        self.writer.write_all(&[0; 4]).map_err(write_error)?;
        self.writer.seek(SeekFrom::Start(0)).map_err(seek_error)?;
        self.writer
            .write_all(b"II")
            .and_then(|()| self.writer.write_all(&42_u16.to_le_bytes()))
            .and_then(|()| {
                let offset = u32::try_from(ifd_offset)
                    .map_err(|_| std::io::Error::other("IFD offset exceeds classic TIFF"))?;
                self.writer.write_all(&offset.to_le_bytes())
            })
            .map_err(write_error)?;
        self.writer.flush().map_err(write_error)
    }
}

/// Builder for the root directory of a [`TiffWriter`].
pub struct TiffDirectory<'a, W> {
    writer: &'a mut TiffWriter<W>,
    entries: Vec<(Tag, Value)>,
}

impl<'a, W> TiffDirectory<'a, W>
where
    W: Write + Seek,
{
    /// Creates a directory builder for `writer`.
    pub const fn new(writer: &'a mut TiffWriter<W>) -> Self {
        Self {
            writer,
            entries: Vec::new(),
        }
    }

    /// Appends image data and returns its absolute file offset.
    pub fn write_data(&mut self, data: &[u8]) -> Result<u64, TiffError> {
        self.writer.write_data(data)
    }

    /// Adds a directory entry.
    pub fn write_tag<T, V>(&mut self, tag: T, value: V) -> Result<(), TiffError>
    where
        T: Into<Tag>,
        V: IntoValue,
    {
        self.entries.push((tag.into(), value.into_tiff_value()));
        Ok(())
    }

    /// Writes the accumulated entries and finalizes the TIFF header.
    pub fn finish(self) -> Result<(), TiffError> { self.writer.finish(self.entries) }
}

struct EncodedEntry {
    tag: Tag,
    field_type: u16,
    count: u32,
    bytes: Vec<u8>,
    offset: Option<u64>,
}

impl EncodedEntry {
    fn new(tag: Tag, value: Value) -> Result<Self, TiffError> {
        let (field_type, count, bytes) = encode_value(&value)?;
        Ok(Self {
            tag,
            field_type,
            count,
            bytes,
            offset: None,
        })
    }
}

fn encode_value(value: &Value) -> Result<(u16, u32, Vec<u8>), TiffError> {
    if let Value::Bytes(bytes) = value {
        let count = u32::try_from(bytes.len())
            .map_err(|_| TiffError::new("TIFF byte count exceeds u32"))?;
        return Ok((1, count, bytes.clone()));
    }
    if let Value::List(values) = value {
        if values.is_empty() {
            return Err(TiffError::new("empty TIFF lists cannot be serialized"));
        }
        let mut encoded = Vec::new();
        let mut field_type = None;
        for value in values {
            let (current_type, current_count, current_bytes) = encode_scalar(value)?;
            if current_count != 1 || field_type.is_some_and(|kind| kind != current_type) {
                return Err(TiffError::new("TIFF lists must contain one scalar type"));
            }
            field_type.get_or_insert(current_type);
            encoded.extend_from_slice(&current_bytes);
        }
        let count = u32::try_from(values.len())
            .map_err(|_| TiffError::new("TIFF list value count exceeds u32"))?;
        return Ok((field_type.expect("the list is non-empty"), count, encoded));
    }
    encode_scalar(value)
}

fn encode_scalar(value: &Value) -> Result<(u16, u32, Vec<u8>), TiffError> {
    let encoded = match value {
        Value::Byte(value) => (1, vec![*value]),
        Value::Ascii(value) => {
            let count = u32::try_from(value.len() + 1)
                .map_err(|_| TiffError::new("ASCII tag exceeds u32 count"))?;
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            return Ok((2, count, bytes));
        }
        Value::Short(value) => (3, value.to_le_bytes().to_vec()),
        Value::Unsigned(value) => (4, value.to_le_bytes().to_vec()),
        Value::Rational(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (5, bytes)
        }
        Value::SignedByte(value) => (6, vec![*value as u8]),
        Value::SignedShort(value) => (8, value.to_le_bytes().to_vec()),
        Value::Signed(value) => (9, value.to_le_bytes().to_vec()),
        Value::SRational(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (10, bytes)
        }
        Value::Float(value) => (11, value.to_le_bytes().to_vec()),
        Value::Double(value) => (12, value.to_le_bytes().to_vec()),
        Value::Ifd(value) => (13, value.to_le_bytes().to_vec()),
        Value::UnsignedBig(value) => (16, value.to_le_bytes().to_vec()),
        Value::SignedBig(value) => (17, value.to_le_bytes().to_vec()),
        Value::IfdBig(value) => (18, value.to_le_bytes().to_vec()),
        Value::RationalBig(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (5, bytes)
        }
        Value::SRationalBig(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (10, bytes)
        }
        Value::Bytes(_) => {
            return Err(TiffError::new("nested TIFF byte arrays are unsupported"));
        }
        Value::List(_) => return Err(TiffError::new("nested TIFF lists are unsupported")),
    };
    Ok((encoded.0, 1, encoded.1))
}

fn write_error(error: std::io::Error) -> TiffError {
    TiffError::new(format!("writing TIFF failed: {error}"))
}

fn seek_error(error: std::io::Error) -> TiffError {
    TiffError::new(format!("seeking TIFF failed: {error}"))
}

#[cfg(feature = "raster")]
mod image_write {
    use std::io::{Seek, Write};

    use fornax_codec::{
        ByteOrder, EncodeCompression, EncodeRequest, ICodecEncode, Predictor, SampleBuffer,
    };
    use fornax_core::MonoImage;

    use crate::constants::{
        CompressionMethod, PhotometricInterpretation, PlanarConfiguration, SampleFormatTag, TiffTag,
    };
    use crate::image::{deinterleave_planes, interleave_planes};
    use crate::{TiffDirectory, TiffError, TiffImage, TiffIoError, TiffWriter, TypedTiffImage};

    /// Strip or tile storage configuration for [`TiffWriteOptions`].
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum WriteStorage {
        /// Store image rows in strips. `None` writes the entire image as one
        /// strip.
        Strips {
            /// Number of rows per strip, or `None` for a single strip.
            rows_per_strip: Option<u32>,
        },
        /// Store image samples in rectangular tiles.
        Tiles {
            /// Tile width in pixels.
            width: u32,
            /// Tile height in pixels.
            height: u32,
        },
    }

    /// Configuration for writing a [`TiffImage`].
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct TiffWriteOptions {
        /// Compression algorithm applied to each strip or tile.
        pub compression: EncodeCompression,
        /// Strip or tile chunk layout.
        pub storage: WriteStorage,
        /// Interleaved (`Chunky`) or separated (`Planar`) sample storage.
        pub planar_configuration: PlanarConfiguration,
        /// Reversible predictor applied before compression.
        ///
        /// When [`Predictor::HorizontalInteger`] is selected with
        /// [`PlanarConfiguration::Chunky`] and `samples_per_pixel > 1`, the
        /// writer automatically uses channel-strided horizontal differencing.
        pub predictor: Predictor,
        /// Explicit photometric interpretation override, or `None` to infer
        /// from channel count.
        pub photometric: Option<PhotometricInterpretation>,
    }

    impl Default for TiffWriteOptions {
        fn default() -> Self {
            Self {
                compression: EncodeCompression::Uncompressed,
                storage: WriteStorage::Strips {
                    rows_per_strip: None,
                },
                planar_configuration: PlanarConfiguration::Chunky,
                predictor: Predictor::None,
                photometric: None,
            }
        }
    }

    impl TiffWriteOptions {
        /// Sets the compression algorithm.
        pub const fn with_compression(mut self, compression: EncodeCompression) -> Self {
            self.compression = compression;
            self
        }

        /// Sets the strip or tile storage layout.
        pub const fn with_storage(mut self, storage: WriteStorage) -> Self {
            self.storage = storage;
            self
        }

        /// Sets the planar configuration.
        pub const fn with_planar_configuration(mut self, planar: PlanarConfiguration) -> Self {
            self.planar_configuration = planar;
            self
        }

        /// Sets the predictor.
        pub const fn with_predictor(mut self, predictor: Predictor) -> Self {
            self.predictor = predictor;
            self
        }

        /// Sets an explicit photometric interpretation.
        pub const fn with_photometric(mut self, photometric: PhotometricInterpretation) -> Self {
            self.photometric = Some(photometric);
            self
        }
    }

    impl<W> TiffWriter<W>
    where
        W: Write + Seek,
    {
        /// Encodes and writes a [`TiffImage`] as the root directory and
        /// finalizes the container.
        pub fn write_image(
            mut self,
            image: &TiffImage,
            options: &TiffWriteOptions,
        ) -> Result<(), TiffIoError> {
            let width = image.width();
            let height = image.height();
            let samples_per_pixel = image.samples_per_pixel();
            if width == 0 || height == 0 {
                return Err(TiffIoError::Invalid(
                    "image dimensions must not be zero".into(),
                ));
            }
            if samples_per_pixel == 0 {
                return Err(TiffIoError::Invalid(
                    "image must contain at least one plane".into(),
                ));
            }

            let (chunk_width, chunk_height, is_tiles) = match options.storage {
                WriteStorage::Strips { rows_per_strip } => {
                    let rows = rows_per_strip.unwrap_or(height);
                    if rows == 0 {
                        return Err(TiffIoError::Invalid(
                            "rows_per_strip must not be zero".into(),
                        ));
                    }
                    (width, rows, false)
                }
                WriteStorage::Tiles {
                    width: tw,
                    height: th,
                } => {
                    if tw == 0 || th == 0 {
                        return Err(TiffIoError::Invalid(
                            "tile dimensions must not be zero".into(),
                        ));
                    }
                    (tw, th, true)
                }
            };

            let planar = match options.planar_configuration {
                PlanarConfiguration::Chunky => PlanarConfiguration::Chunky,
                PlanarConfiguration::Planar => PlanarConfiguration::Planar,
                PlanarConfiguration::Unknown(id) => {
                    return Err(TiffIoError::Unsupported(
                        format!("unsupported planar configuration {id}").into(),
                    ));
                }
            };

            let (bits_per_sample, sample_format_tag, encoded_chunks) =
                encode_tiff_image(image, chunk_width, chunk_height, planar, options)?;

            let mut offsets = Vec::with_capacity(encoded_chunks.len());
            let mut byte_counts = Vec::with_capacity(encoded_chunks.len());
            for chunk in encoded_chunks {
                let offset = self.write_data(&chunk)?;
                let offset = u32::try_from(offset)
                    .map_err(|_| TiffError::new("classic TIFF chunk offset exceeds u32"))?;
                let byte_count = u32::try_from(chunk.len())
                    .map_err(|_| TiffError::new("classic TIFF chunk byte count exceeds u32"))?;
                offsets.push(offset);
                byte_counts.push(byte_count);
            }

            let photometric = options.photometric.unwrap_or(if samples_per_pixel <= 2 {
                PhotometricInterpretation::BlackIsZero
            } else {
                PhotometricInterpretation::Rgb
            });

            let mut dir = TiffDirectory::new(&mut self);
            dir.write_tag(TiffTag::ImageWidth, width)?;
            dir.write_tag(TiffTag::ImageLength, height)?;

            let spp_u16 = u16::try_from(samples_per_pixel)
                .map_err(|_| TiffIoError::Invalid("samples_per_pixel exceeds u16".into()))?;
            dir.write_tag(TiffTag::SamplesPerPixel, spp_u16)?;
            if samples_per_pixel == 1 {
                dir.write_tag(TiffTag::BitsPerSample, bits_per_sample)?;
                dir.write_tag(TiffTag::SampleFormat, u16::from(sample_format_tag))?;
            } else {
                let bits = vec![bits_per_sample; samples_per_pixel];
                let formats = vec![u16::from(sample_format_tag); samples_per_pixel];
                dir.write_tag(TiffTag::BitsPerSample, bits.as_slice())?;
                dir.write_tag(TiffTag::SampleFormat, formats.as_slice())?;
            }

            dir.write_tag(
                TiffTag::Compression,
                u16::from(compression_method(options.compression)),
            )?;
            dir.write_tag(TiffTag::PhotometricInterpretation, u16::from(photometric))?;
            dir.write_tag(TiffTag::PlanarConfiguration, u16::from(planar))?;

            if matches!(
                image,
                TiffImage::U8(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
                    | TiffImage::U16(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
                    | TiffImage::U32(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
                    | TiffImage::F32(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
                    | TiffImage::F64(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
            ) {
                // Unassociated alpha (2) is the standard default for general
                // RGBA/GA.
                dir.write_tag(TiffTag::ExtraSamples, 2_u16)?;
            }

            let predictor_tag = match options.predictor {
                Predictor::None => 1_u16,
                Predictor::HorizontalInteger | Predictor::HorizontalIntegerStrided { .. } => 2_u16,
                Predictor::FloatingPoint { .. } => 3_u16,
            };
            if predictor_tag != 1 {
                dir.write_tag(TiffTag::Predictor, predictor_tag)?;
            }

            if is_tiles {
                dir.write_tag(TiffTag::TileWidth, chunk_width)?;
                dir.write_tag(TiffTag::TileLength, chunk_height)?;
                dir.write_tag(TiffTag::TileOffsets, offsets.as_slice())?;
                dir.write_tag(TiffTag::TileByteCounts, byte_counts.as_slice())?;
            } else {
                dir.write_tag(TiffTag::RowsPerStrip, chunk_height)?;
                dir.write_tag(TiffTag::StripOffsets, offsets.as_slice())?;
                dir.write_tag(TiffTag::StripByteCounts, byte_counts.as_slice())?;
            }

            dir.finish()?;
            Ok(())
        }
    }

    fn encode_tiff_image(
        image: &TiffImage,
        chunk_width: u32,
        chunk_height: u32,
        planar: PlanarConfiguration,
        options: &TiffWriteOptions,
    ) -> Result<(u16, SampleFormatTag, Vec<Vec<u8>>), TiffIoError> {
        let cw = usize::try_from(chunk_width)
            .map_err(|_| TiffIoError::Invalid("chunk_width exceeds usize".into()))?;
        let ch = usize::try_from(chunk_height)
            .map_err(|_| TiffIoError::Invalid("chunk_height exceeds usize".into()))?;

        macro_rules! encode_typed {
            ($typed:expr, $bits:expr, $format:expr, $ctor:expr) => {{
                let samples_per_pixel = $typed.samples_per_pixel();
                let predictor = encode_predictor(options.predictor, planar, samples_per_pixel);
                match planar {
                    PlanarConfiguration::Chunky => {
                        let w = $typed.width();
                        let h = $typed.height();
                        let interleaved_w = w
                            .checked_mul(u32::try_from(samples_per_pixel).map_err(|_| {
                                TiffIoError::Invalid("samples_per_pixel exceeds u32".into())
                            })?)
                            .ok_or_else(|| {
                                TiffIoError::Invalid("interleaved image width exceeds u32".into())
                            })?;
                        let interleaved_cw =
                            cw.checked_mul(samples_per_pixel).ok_or_else(|| {
                                TiffIoError::Invalid("interleaved chunk width exceeds usize".into())
                            })?;
                        let raw = typed_to_interleaved($typed)?;
                        let buf = $ctor(interleaved_w, h, raw)?;
                        let encoded = options.compression.encode(&EncodeRequest {
                            image: &buf,
                            chunk_width: interleaved_cw,
                            chunk_height: ch,
                            byte_order: ByteOrder::Little,
                            predictor,
                        })?;
                        Ok(($bits, $format, encoded.chunks))
                    }
                    PlanarConfiguration::Planar => {
                        let planes = typed_to_planes($typed)?;
                        let mut all_chunks = Vec::new();
                        for plane in planes {
                            let buf = $ctor(plane.width(), plane.height(), plane.into_raw())?;
                            let encoded = options.compression.encode(&EncodeRequest {
                                image: &buf,
                                chunk_width: cw,
                                chunk_height: ch,
                                byte_order: ByteOrder::Little,
                                predictor,
                            })?;
                            all_chunks.extend(encoded.chunks);
                        }
                        Ok(($bits, $format, all_chunks))
                    }
                    PlanarConfiguration::Unknown(_) => unreachable!(),
                }
            }};
        }

        match image {
            TiffImage::U8(typed) => {
                encode_typed!(typed, 8, SampleFormatTag::Unsigned, SampleBuffer::from_u8)
            }
            TiffImage::U16(typed) => {
                encode_typed!(typed, 16, SampleFormatTag::Unsigned, SampleBuffer::from_u16)
            }
            TiffImage::U32(typed) => {
                encode_typed!(typed, 32, SampleFormatTag::Unsigned, SampleBuffer::from_u32)
            }
            TiffImage::F32(typed) => {
                encode_typed!(typed, 32, SampleFormatTag::Float, SampleBuffer::from_f32)
            }
            TiffImage::F64(typed) => {
                encode_typed!(typed, 64, SampleFormatTag::Float, SampleBuffer::from_f64)
            }
        }
    }

    fn encode_predictor(
        predictor: Predictor,
        planar: PlanarConfiguration,
        samples_per_pixel: usize,
    ) -> Predictor {
        match predictor {
            Predictor::HorizontalInteger
                if planar == PlanarConfiguration::Chunky && samples_per_pixel > 1 =>
            {
                Predictor::HorizontalIntegerStrided { samples_per_pixel }
            }
            Predictor::FloatingPoint { .. } => Predictor::FloatingPoint {
                byte_stride: if planar == PlanarConfiguration::Planar {
                    1
                } else {
                    samples_per_pixel
                },
            },
            other => other,
        }
    }

    fn typed_to_interleaved<T: image::Primitive>(
        typed: &TypedTiffImage<T>,
    ) -> Result<Vec<T>, TiffIoError> {
        match typed {
            TypedTiffImage::Mono(img) => Ok(img.as_raw().clone()),
            TypedTiffImage::Uv(img) => Ok(img.as_raw().clone()),
            TypedTiffImage::Rgb(img) => Ok(img.as_raw().clone()),
            TypedTiffImage::Rgba(img) => Ok(img.as_raw().clone()),
            TypedTiffImage::MultiPlane(planes) => {
                validate_multiplane_dimensions(planes)?;
                Ok(interleave_planes(planes))
            }
        }
    }

    fn typed_to_planes<T: image::Primitive>(
        typed: &TypedTiffImage<T>,
    ) -> Result<Vec<MonoImage<T>>, TiffIoError> {
        match typed {
            TypedTiffImage::Mono(img) => Ok(vec![img.clone()]),
            TypedTiffImage::Uv(img) => {
                deinterleave_planes(img.width(), img.height(), 2, img.as_raw())
            }
            TypedTiffImage::Rgb(img) => {
                deinterleave_planes(img.width(), img.height(), 3, img.as_raw())
            }
            TypedTiffImage::Rgba(img) => {
                deinterleave_planes(img.width(), img.height(), 4, img.as_raw())
            }
            TypedTiffImage::MultiPlane(planes) => {
                validate_multiplane_dimensions(planes)?;
                Ok(planes.clone())
            }
        }
    }

    fn validate_multiplane_dimensions<T: image::Primitive>(
        planes: &[MonoImage<T>],
    ) -> Result<(), TiffIoError> {
        let first = planes
            .first()
            .ok_or_else(|| TiffIoError::Invalid("multi-plane image has no planes".into()))?;
        let dims = first.dimensions();
        if dims.0 == 0 || dims.1 == 0 {
            return Err(TiffIoError::Invalid(
                "multi-plane image dimensions must not be zero".into(),
            ));
        }
        if planes.iter().any(|plane| plane.dimensions() != dims) {
            return Err(TiffIoError::Invalid(
                "all planes in a multi-plane image must share the same dimensions".into(),
            ));
        }
        Ok(())
    }

    const fn compression_method(compression: EncodeCompression) -> CompressionMethod {
        match compression {
            EncodeCompression::Uncompressed => CompressionMethod::Uncompressed,
            EncodeCompression::LosslessJpeg => CompressionMethod::LosslessJpeg,
            EncodeCompression::Deflate => CompressionMethod::Deflate,
            #[cfg(feature = "lzw")]
            EncodeCompression::Lzw => CompressionMethod::Lzw,
            #[cfg(feature = "packbits")]
            EncodeCompression::PackBits => CompressionMethod::PackBits,
            #[cfg(feature = "zstd")]
            EncodeCompression::Zstd { .. } => CompressionMethod::Zstd,
        }
    }
}

#[cfg(feature = "raster")]
pub use image_write::{TiffWriteOptions, WriteStorage};

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{Tag, TiffWriter, Value, parse};

    #[test]
    fn writer_round_trips_inline_and_external_values() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = TiffWriter::new(&mut output).unwrap();
        let data_offset = writer.write_data(b"pixels").unwrap();
        writer
            .finish([
                (Tag(256), Value::Unsigned(2)),
                (
                    Tag(273),
                    Value::List(vec![
                        Value::Unsigned(u32::try_from(data_offset).unwrap()),
                        Value::Unsigned(u32::try_from(data_offset).unwrap()),
                    ]),
                ),
                (Tag(270), Value::Ascii("Fornax".into())),
            ])
            .unwrap();

        let parsed = parse(output.get_ref()).unwrap();
        assert_eq!(parsed.root().get(&Tag(256)), Some(&Value::Unsigned(2)));
        assert_eq!(
            parsed.root().get(&Tag(270)),
            Some(&Value::Ascii("Fornax".into()))
        );
        let Value::List(offsets) = parsed.root().get(&Tag(273)).unwrap() else {
            panic!("strip offsets should remain a list");
        };
        assert_eq!(offsets.len(), 2);
    }
}
