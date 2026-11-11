use crate::error::{BmffError, invalid, limited};

/// Sample-table entry and allocation caps.
#[derive(Clone, Copy, Debug)]
pub struct SampleTableLimits {
    /// Maximum entries in one table.
    pub max_entries: usize,
    /// Maximum samples represented by one table.
    pub max_samples: u64,
    /// Maximum samples in one chunk.
    pub max_samples_per_chunk: u32,
}

impl Default for SampleTableLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_samples: 16_777_216,
            max_samples_per_chunk: 1_000_000,
        }
    }
}

/// Byte range of one sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sample {
    /// Offset in the BMFF source.
    pub offset: usize,
    /// Encoded sample size.
    pub size: usize,
}

/// Chunk offset table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChunkOffsets {
    /// 32-bit chunk offsets from `stco`.
    Co32(Vec<u32>),
    /// 64-bit chunk offsets from `co64`.
    Co64(Vec<u64>),
}

impl ChunkOffsets {
    /// Parses either `stco` or `co64` payload.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported type, truncation, or count
    /// overflow.
    pub fn parse(
        typ: [u8; 4],
        payload: &[u8],
        limits: SampleTableLimits,
    ) -> Result<Self, BmffError> {
        let mut reader = Reader::new(payload, limits);
        reader.full_box()?;
        let count = reader.entry_count()?;
        match &typ {
            b"stco" => Ok(Self::Co32(reader.u32_entries(count)?)),
            b"co64" => Ok(Self::Co64(reader.u64_entries(count)?)),
            _ => Err(invalid("chunk offset box must be stco or co64")),
        }
    }

    fn get(&self, index: usize) -> Option<u64> {
        match self {
            Self::Co32(values) => values.get(index).copied().map(u64::from),
            Self::Co64(values) => values.get(index).copied(),
        }
    }

    const fn len(&self) -> usize {
        match self {
            Self::Co32(values) => values.len(),
            Self::Co64(values) => values.len(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct StscEntry {
    first_chunk: u32,
    samples_per_chunk: u32,
}

/// Resolved BMFF sample table.
#[derive(Clone, Debug)]
pub struct SampleTable {
    stsc: Vec<StscEntry>,
    constant_sample_size: u32,
    sample_sizes: Vec<u32>,
    chunk_offsets: ChunkOffsets,
    sample_count: u32,
}

impl SampleTable {
    /// Parses the three required sample-table payloads.
    ///
    /// # Errors
    ///
    /// Returns an error for truncated data, invalid mappings, or exceeded
    /// limits.
    pub fn parse(
        stsc: &[u8],
        stsz: &[u8],
        chunk_offsets_type: [u8; 4],
        chunk_offsets: &[u8],
        limits: SampleTableLimits,
    ) -> Result<Self, BmffError> {
        let mut stsc_reader = Reader::new(stsc, limits);
        stsc_reader.full_box()?;
        let stsc_count = stsc_reader.entry_count()?;
        let mut entries = Vec::with_capacity(stsc_count);
        let mut previous_chunk = 0_u32;
        for _ in 0..stsc_count {
            let first_chunk = stsc_reader.u32()?;
            let samples_per_chunk = stsc_reader.u32()?;
            let sample_description_index = stsc_reader.u32()?;
            if first_chunk == 0
                || first_chunk <= previous_chunk
                || samples_per_chunk == 0
                || samples_per_chunk > limits.max_samples_per_chunk
                || sample_description_index == 0
            {
                return Err(invalid("invalid stsc entry"));
            }
            previous_chunk = first_chunk;
            entries.push(StscEntry {
                first_chunk,
                samples_per_chunk,
            });
        }
        if entries.first().is_some_and(|entry| entry.first_chunk != 1) {
            return Err(invalid("first stsc entry must begin at chunk 1"));
        }

        let mut stsz_reader = Reader::new(stsz, limits);
        stsz_reader.full_box()?;
        let constant_sample_size = stsz_reader.u32()?;
        let sample_count = stsz_reader.u32()?;
        if u64::from(sample_count) > limits.max_samples {
            return Err(limited("stsz sample count exceeds configured maximum"));
        }
        let sample_sizes = if constant_sample_size == 0 {
            stsz_reader.u32_entries(
                usize::try_from(sample_count)
                    .map_err(|_| limited("stsz sample count exceeds address space"))?,
            )?
        } else {
            Vec::new()
        };

        let chunk_offsets = ChunkOffsets::parse(chunk_offsets_type, chunk_offsets, limits)?;
        let table = Self {
            stsc: entries,
            constant_sample_size,
            sample_sizes,
            chunk_offsets,
            sample_count,
        };
        table.validate()?;
        Ok(table)
    }

    /// Returns the number of samples.
    pub const fn sample_count(&self) -> u32 { self.sample_count }

    /// Resolves a zero-based sample index to a bounded source byte range.
    ///
    /// # Errors
    ///
    /// Returns [`BmffError::InvalidInput`] when the index is out of range or
    /// the resolved sample extends beyond `source_len`.
    pub fn sample(&self, index: usize, source_len: usize) -> Result<Sample, BmffError> {
        let index = u64::try_from(index).map_err(|_| invalid("sample index exceeds u64"))?;
        if index >= u64::from(self.sample_count) {
            return Err(invalid(format!(
                "sample index {index} exceeds count {}",
                self.sample_count
            )));
        }
        let target = index + 1;
        let mut samples_seen = 0_u64;
        for (position, entry) in self.stsc.iter().enumerate() {
            let end_chunk = self.stsc.get(position + 1).map_or_else(
                || u64::from(u32::MAX),
                |next| u64::from(next.first_chunk) - 1,
            );
            let start_chunk = u64::from(entry.first_chunk);
            let chunks = end_chunk
                .saturating_sub(start_chunk)
                .saturating_add(1)
                .min(u64::try_from(self.chunk_offsets.len()).unwrap_or(u64::MAX));
            let samples = chunks
                .checked_mul(u64::from(entry.samples_per_chunk))
                .ok_or_else(|| invalid("stsc sample count overflow"))?;
            if target <= samples_seen + samples {
                let within = target - samples_seen - 1;
                let chunk = start_chunk + within / u64::from(entry.samples_per_chunk);
                let index_in_chunk = u32::try_from(within % u64::from(entry.samples_per_chunk))
                    .map_err(|_| invalid("chunk sample index exceeds u32"))?;
                let chunk_usize =
                    usize::try_from(chunk - 1).map_err(|_| invalid("chunk index exceeds usize"))?;
                let chunk_offset = self
                    .chunk_offsets
                    .get(chunk_usize)
                    .ok_or_else(|| invalid("stsc references a missing chunk offset"))?;
                let sample_number = samples_seen + within + 1;
                let relative = self.relative_offset(sample_number, index_in_chunk)?;
                let offset = chunk_offset
                    .checked_add(relative)
                    .ok_or_else(|| invalid("sample offset overflow"))?;
                let size = self.sample_size(sample_number)?;
                let offset_usize =
                    usize::try_from(offset).map_err(|_| invalid("sample offset exceeds usize"))?;
                let size_usize =
                    usize::try_from(size).map_err(|_| invalid("sample size exceeds usize"))?;
                let end = offset_usize
                    .checked_add(size_usize)
                    .ok_or_else(|| invalid("sample end overflow"))?;
                if end > source_len {
                    return Err(invalid("sample extends beyond BMFF source"));
                }
                return Ok(Sample {
                    offset: offset_usize,
                    size: size_usize,
                });
            }
            samples_seen += samples;
        }
        Err(invalid("stsc mappings do not cover the requested sample"))
    }

    fn validate(&self) -> Result<(), BmffError> {
        let mut samples = 0_u64;
        for (position, entry) in self.stsc.iter().enumerate() {
            let end_chunk = self.stsc.get(position + 1).map_or_else(
                || u64::try_from(self.chunk_offsets.len()).unwrap_or(u64::MAX),
                |next| u64::from(next.first_chunk) - 1,
            );
            let chunks = end_chunk
                .checked_sub(u64::from(entry.first_chunk))
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| invalid("stsc chunk range is invalid"))?;
            samples = samples
                .checked_add(
                    chunks
                        .checked_mul(u64::from(entry.samples_per_chunk))
                        .ok_or_else(|| invalid("stsc sample count overflow"))?,
                )
                .ok_or_else(|| invalid("stsc sample count overflow"))?;
        }
        if samples != u64::from(self.sample_count) {
            return Err(invalid(format!(
                "stsc maps {samples} samples but stsz declares {}",
                self.sample_count
            )));
        }
        Ok(())
    }

    fn sample_size(&self, sample_number: u64) -> Result<u32, BmffError> {
        if self.constant_sample_size != 0 {
            return Ok(self.constant_sample_size);
        }
        let index = usize::try_from(sample_number - 1)
            .map_err(|_| invalid("sample number exceeds usize"))?;
        self.sample_sizes
            .get(index)
            .copied()
            .ok_or_else(|| invalid("sample size is missing"))
    }

    fn relative_offset(&self, sample_number: u64, index_in_chunk: u32) -> Result<u64, BmffError> {
        if self.constant_sample_size != 0 {
            return u64::from(self.constant_sample_size)
                .checked_mul(u64::from(index_in_chunk))
                .ok_or_else(|| invalid("sample offset overflow"));
        }
        let first = sample_number - u64::from(index_in_chunk);
        (0..index_in_chunk).try_fold(0_u64, |total, offset| {
            total
                .checked_add(u64::from(self.sample_size(first + u64::from(offset))?))
                .ok_or_else(|| invalid("sample offset overflow"))
        })
    }
}

struct Reader<'a> {
    source: &'a [u8],
    position: usize,
    limits: SampleTableLimits,
}

impl<'a> Reader<'a> {
    const fn new(source: &'a [u8], limits: SampleTableLimits) -> Self {
        Self {
            source,
            position: 0,
            limits,
        }
    }

    fn full_box(&mut self) -> Result<(u8, u32), BmffError> {
        let bytes: [u8; 4] = self
            .bytes(4)?
            .try_into()
            .map_err(|_| invalid("FullBox header is truncated"))?;
        Ok((
            bytes[0],
            u32::from_be_bytes([0, bytes[1], bytes[2], bytes[3]]),
        ))
    }

    fn entry_count(&mut self) -> Result<usize, BmffError> {
        let count = usize::try_from(self.u32()?)
            .map_err(|_| invalid("sample-table entry count exceeds address space"))?;
        if count > self.limits.max_entries {
            return Err(limited(format!(
                "sample-table entry count {count} exceeds {}",
                self.limits.max_entries
            )));
        }
        Ok(count)
    }

    fn u32_entries(&mut self, count: usize) -> Result<Vec<u32>, BmffError> {
        (0..count).map(|_| self.u32()).collect()
    }

    fn u64_entries(&mut self, count: usize) -> Result<Vec<u64>, BmffError> {
        (0..count).map(|_| self.u64()).collect()
    }

    fn u32(&mut self) -> Result<u32, BmffError> {
        let bytes: [u8; 4] = self
            .bytes(4)?
            .try_into()
            .map_err(|_| invalid("u32 is truncated"))?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, BmffError> {
        let bytes: [u8; 8] = self
            .bytes(8)?
            .try_into()
            .map_err(|_| invalid("u64 is truncated"))?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn bytes(&mut self, count: usize) -> Result<&'a [u8], BmffError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("sample-table position overflow"))?;
        let bytes = self
            .source
            .get(self.position..end)
            .ok_or_else(|| invalid("sample table is truncated"))?;
        self.position = end;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_offset_box(entries: &[u32]) -> Vec<u8> {
        let mut bytes = vec![0; 4];
        bytes.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_be_bytes());
        for entry in entries {
            bytes.extend_from_slice(&entry.to_be_bytes());
        }
        bytes
    }

    fn stsc_box(entries: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut bytes = vec![0; 4];
        bytes.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_be_bytes());
        for (first_chunk, samples_per_chunk, description) in entries {
            bytes.extend_from_slice(&first_chunk.to_be_bytes());
            bytes.extend_from_slice(&samples_per_chunk.to_be_bytes());
            bytes.extend_from_slice(&description.to_be_bytes());
        }
        bytes
    }

    fn stsz_box(constant_size: u32, entries: &[u32]) -> Vec<u8> {
        let mut bytes = vec![0; 4];
        bytes.extend_from_slice(&constant_size.to_be_bytes());
        bytes.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_be_bytes());
        if constant_size == 0 {
            for entry in entries {
                bytes.extend_from_slice(&entry.to_be_bytes());
            }
        }
        bytes
    }

    #[test]
    fn resolves_variable_sizes_and_chunk_offsets() {
        let stsc = stsc_box(&[(1, 2, 1), (2, 1, 1)]);
        let stsz = stsz_box(0, &[2, 3, 4]);
        let stco = chunk_offset_box(&[100, 200]);

        let table = SampleTable::parse(&stsc, &stsz, *b"stco", &stco, SampleTableLimits::default())
            .unwrap();
        assert_eq!(
            table.sample(0, 300).unwrap(),
            Sample {
                offset: 100,
                size: 2
            }
        );
        assert_eq!(
            table.sample(1, 300).unwrap(),
            Sample {
                offset: 102,
                size: 3
            }
        );
        assert_eq!(
            table.sample(2, 300).unwrap(),
            Sample {
                offset: 200,
                size: 4
            }
        );
    }

    #[test]
    fn rejects_out_of_bounds_and_malformed_tables() {
        let stsc = stsc_box(&[(1, 1, 1)]);
        let stsz = stsz_box(4, &[4]);
        let stco = chunk_offset_box(&[8]);

        let table = SampleTable::parse(&stsc, &stsz, *b"stco", &stco, SampleTableLimits::default())
            .unwrap();
        assert!(matches!(
            table.sample(0, 11),
            Err(BmffError::InvalidInput(_))
        ));
        assert!(matches!(
            table.sample(1, 100),
            Err(BmffError::InvalidInput(_))
        ));
        assert!(matches!(
            SampleTable::parse(
                &stsc[..10],
                &stsz,
                *b"stco",
                &stco,
                SampleTableLimits::default()
            ),
            Err(BmffError::InvalidInput(_))
        ));
    }

    #[test]
    fn rejects_entry_and_sample_count_caps() {
        let stsc = stsc_box(&[(1, 1, 1)]);
        let stsz = stsz_box(4, &[4]);
        let stco = chunk_offset_box(&[8]);

        let entry_limited = SampleTableLimits {
            max_entries: 0,
            ..SampleTableLimits::default()
        };
        assert!(matches!(
            SampleTable::parse(&stsc, &stsz, *b"stco", &stco, entry_limited),
            Err(BmffError::LimitExceeded(_))
        ));

        let sample_limited = SampleTableLimits {
            max_samples: 0,
            ..SampleTableLimits::default()
        };
        assert!(matches!(
            SampleTable::parse(&stsc, &stsz, *b"stco", &stco, sample_limited),
            Err(BmffError::LimitExceeded(_))
        ));
    }
}
