use std::fmt;

use crate::error::{BmffError, invalid, limited};

/// Four ISO-BMFF type bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FourCc([u8; 4]);

impl FourCc {
    /// Creates a four-character-code from its four bytes.
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Returns the underlying bytes.
    pub const fn as_bytes(self) -> [u8; 4] {
        self.0
    }
}

impl fmt::Display for FourCc {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&String::from_utf8_lossy(&self.0))
    }
}

/// Parser allocation, traversal, and sample-table limits.
#[derive(Clone, Copy, Debug)]
pub struct BmffLimits {
    /// Maximum bytes accepted for one box.
    pub max_box_size: u64,
    /// Maximum number of boxes visited by one parse or lookup.
    pub max_boxes: usize,
    /// Maximum child nesting depth.
    pub max_depth: u8,
}

impl Default for BmffLimits {
    fn default() -> Self {
        Self {
            max_box_size: 1 << 40,
            max_boxes: 100_000,
            max_depth: 32,
        }
    }
}

/// One ISO-BMFF box header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoxHeader {
    offset: u64,
    size: u64,
    header_size: u64,
    typ: FourCc,
    uuid: Option<[u8; 16]>,
}

impl BoxHeader {
    /// Returns the byte offset of the box header.
    pub const fn offset(self) -> u64 {
        self.offset
    }

    /// Returns the complete box size including its header.
    pub const fn size(self) -> u64 {
        self.size
    }

    /// Returns the header size, including extended size and UUID fields.
    pub const fn header_size(self) -> u64 {
        self.header_size
    }

    /// Returns the box type.
    pub const fn typ(self) -> FourCc {
        self.typ
    }

    /// Returns the UUID when `typ` is `uuid`.
    pub const fn uuid(self) -> Option<[u8; 16]> {
        self.uuid
    }

    /// Returns the exclusive end offset.
    pub const fn end(self) -> u64 {
        self.offset + self.size
    }

    /// Returns the payload byte offset.
    pub const fn payload_offset(self) -> u64 {
        self.offset + self.header_size
    }
}

/// A box header and borrowed payload.
#[derive(Clone, Copy, Debug)]
pub struct BoxRef<'a> {
    header: BoxHeader,
    payload: &'a [u8],
}

impl<'a> BoxRef<'a> {
    /// Returns the box header.
    pub const fn header(self) -> BoxHeader {
        self.header
    }

    /// Returns the box type.
    pub const fn typ(self) -> FourCc {
        self.header.typ
    }

    /// Returns the UUID bytes when this is a UUID box.
    pub const fn uuid(self) -> Option<[u8; 16]> {
        self.header.uuid
    }

    /// Returns the payload after all headers.
    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }

    /// Parses the payload as a FullBox version and flags.
    ///
    /// # Errors
    ///
    /// Returns [`BmffError::InvalidInput`] when the payload is shorter than four
    /// bytes.
    pub fn full_box(self) -> Result<(u8, u32), BmffError> {
        let bytes = self
            .payload
            .get(..4)
            .ok_or_else(|| invalid("FullBox payload is shorter than four bytes"))?;
        let flags = u32::from_be_bytes([0, bytes[1], bytes[2], bytes[3]]);
        Ok((bytes[0], flags))
    }

    /// Parses direct child boxes from this payload.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed sizes or when limits are exceeded.
    pub fn children(self, limits: BmffLimits) -> Result<Vec<Self>, BmffError> {
        parse_boxes(
            self.payload,
            limits,
            self.header.offset + self.header.header_size,
            0,
        )
    }
}

/// Parsed top-level ISO-BMFF boxes.
#[derive(Clone, Debug)]
pub struct Bmff<'a> {
    source: &'a [u8],
    boxes: Vec<BoxRef<'a>>,
}

impl<'a> Bmff<'a> {
    /// Parses top-level boxes from `source`.
    ///
    /// # Errors
    ///
    /// Returns [`BmffError::InvalidInput`] for truncated or malformed boxes and
    /// [`BmffError::LimitExceeded`] when default limits are exceeded.
    pub fn parse(source: &'a [u8]) -> Result<Self, BmffError> {
        Self::parse_with_limits(source, BmffLimits::default())
    }

    /// Parses top-level boxes using explicit limits.
    ///
    /// # Errors
    ///
    /// Returns [`BmffError::InvalidInput`] for truncated or malformed boxes and
    /// [`BmffError::LimitExceeded`] when `limits` are exceeded.
    pub fn parse_with_limits(source: &'a [u8], limits: BmffLimits) -> Result<Self, BmffError> {
        let boxes = parse_boxes(source, limits, 0, 0)?;
        Ok(Self { source, boxes })
    }

    /// Returns the complete source bytes.
    pub const fn source(&self) -> &'a [u8] {
        self.source
    }

    /// Returns the parsed top-level boxes.
    pub fn boxes(&self) -> &[BoxRef<'a>] {
        &self.boxes
    }

    /// Finds the first box matching a top-down path.
    ///
    /// # Errors
    ///
    /// Returns an error when a traversed child payload is malformed or limits
    /// are exceeded.
    pub fn find(&self, path: &[FourCc]) -> Result<Option<BoxRef<'a>>, BmffError> {
        self.find_with_limits(path, &BmffLimits::default())
    }

    /// Finds a box path using explicit limits.
    ///
    /// # Errors
    ///
    /// Returns an error when a traversed child payload is malformed or limits
    /// are exceeded.
    pub fn find_with_limits(
        &self,
        path: &[FourCc],
        limits: &BmffLimits,
    ) -> Result<Option<BoxRef<'a>>, BmffError> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(None);
        };
        let Some(mut current) = self.boxes.iter().copied().find(|item| item.typ() == *first) else {
            return Ok(None);
        };
        let mut visited = 1_usize;
        for (depth, typ) in rest.iter().enumerate() {
            if depth >= usize::from(limits.max_depth) {
                return Err(limited(format!(
                    "box lookup exceeds maximum depth {}",
                    limits.max_depth
                )));
            }
            let children = current.children(*limits)?;
            visited = visited
                .checked_add(children.len())
                .ok_or_else(|| limited("BMFF lookup box count overflow"))?;
            if visited > limits.max_boxes {
                return Err(limited(format!(
                    "box lookup exceeds maximum count {}",
                    limits.max_boxes
                )));
            }
            let Some(child) = children.into_iter().find(|item| item.typ() == *typ) else {
                return Ok(None);
            };
            current = child;
        }
        Ok(Some(current))
    }
}

fn parse_boxes(
    source: &[u8],
    limits: BmffLimits,
    base_offset: u64,
    depth: u8,
) -> Result<Vec<BoxRef<'_>>, BmffError> {
    if depth > limits.max_depth {
        return Err(limited(format!(
            "box nesting exceeds maximum depth {}",
            limits.max_depth
        )));
    }
    let mut boxes = Vec::new();
    let mut cursor = 0_usize;
    while cursor < source.len() {
        if boxes.len() >= limits.max_boxes {
            return Err(limited(format!(
                "box count exceeds maximum {}",
                limits.max_boxes
            )));
        }
        let remaining = &source[cursor..];
        let size32 = read_u32(remaining, 0)?;
        let typ_bytes: [u8; 4] = remaining
            .get(4..8)
            .ok_or_else(|| invalid("box header is truncated"))?
            .try_into()
            .map_err(|_| invalid("box type is truncated"))?;
        let typ = FourCc::new(typ_bytes);
        let mut header_size = 8_u64;
        let size = match size32 {
            0 => u64::try_from(remaining.len()).map_err(|_| invalid("box size exceeds u64"))?,
            1 => {
                header_size = 16;
                read_u64(remaining, 8)?
            }
            value => u64::from(value),
        };
        if typ == FourCc::new(*b"uuid") {
            header_size = header_size
                .checked_add(16)
                .ok_or_else(|| invalid("UUID box header overflow"))?;
        }
        if size < header_size {
            return Err(invalid(format!(
                "{typ} box size {size} is smaller than its {header_size}-byte header"
            )));
        }
        if size > limits.max_box_size {
            return Err(limited(format!(
                "{typ} box size {size} exceeds maximum {}",
                limits.max_box_size
            )));
        }
        let size_usize =
            usize::try_from(size).map_err(|_| invalid("box size exceeds address space"))?;
        let header_usize = usize::try_from(header_size)
            .map_err(|_| invalid("box header size exceeds address space"))?;
        let box_bytes = remaining
            .get(..size_usize)
            .ok_or_else(|| invalid(format!("{typ} box is truncated")))?;
        let uuid = if typ == FourCc::new(*b"uuid") {
            Some(
                box_bytes
                    .get(header_usize - 16..header_usize)
                    .ok_or_else(|| invalid("UUID box header is truncated"))?
                    .try_into()
                    .map_err(|_| invalid("UUID box header is truncated"))?,
            )
        } else {
            None
        };
        let offset = base_offset
            .checked_add(u64::try_from(cursor).map_err(|_| invalid("box offset exceeds u64"))?)
            .ok_or_else(|| invalid("box offset overflow"))?;
        boxes.push(BoxRef {
            header: BoxHeader {
                offset,
                size,
                header_size,
                typ,
                uuid,
            },
            payload: box_bytes
                .get(header_usize..)
                .ok_or_else(|| invalid("box payload is truncated"))?,
        });
        cursor = cursor
            .checked_add(size_usize)
            .ok_or_else(|| invalid("box cursor overflow"))?;
    }
    Ok(boxes)
}

fn read_u32(source: &[u8], offset: usize) -> Result<u32, BmffError> {
    let bytes: [u8; 4] = source
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("box header is truncated"))?
        .try_into()
        .map_err(|_| invalid("box header is truncated"))?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_u64(source: &[u8], offset: usize) -> Result<u64, BmffError> {
    let bytes: [u8; 8] = source
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("extended box size is truncated"))?
        .try_into()
        .map_err(|_| invalid("extended box size is truncated"))?;
    Ok(u64::from_be_bytes(bytes))
}
