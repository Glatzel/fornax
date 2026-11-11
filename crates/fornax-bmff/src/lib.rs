//! Bounded ISO Base Media File Format box parsing.

mod boxed;
mod error;
mod sample_table;

pub use boxed::{Bmff, BmffLimits, BoxHeader, BoxRef, FourCc};
pub use error::BmffError;
pub use sample_table::{ChunkOffsets, Sample, SampleTable, SampleTableLimits};

#[cfg(test)]
mod tests {
    use super::*;

    fn box_bytes(typ: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&u32::try_from(payload.len() + 8).unwrap().to_be_bytes());
        bytes.extend_from_slice(typ);
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn parses_32_bit_64_bit_and_to_eof_boxes() {
        let mut source = box_bytes(b"ftyp", b"crx ");
        source.extend_from_slice(&1_u32.to_be_bytes());
        source.extend_from_slice(b"moov");
        source.extend_from_slice(&16_u64.to_be_bytes());
        source.extend_from_slice(&0_u32.to_be_bytes());
        source.extend_from_slice(b"mdat");
        source.extend_from_slice(b"payload");

        let bmff = Bmff::parse(&source).unwrap();
        assert_eq!(bmff.boxes().len(), 3);
        assert_eq!(bmff.boxes()[0].payload(), b"crx ");
        assert!(bmff.boxes()[1].payload().is_empty());
        assert_eq!(bmff.boxes()[2].payload(), b"payload");
    }

    #[test]
    fn exposes_uuid_payload_without_the_uuid_header() {
        let uuid = [0x11; 16];
        let mut payload = uuid.to_vec();
        payload.extend_from_slice(b"raw");
        let source = box_bytes(b"uuid", &payload);

        let parsed = Bmff::parse(&source).unwrap();
        assert_eq!(parsed.boxes()[0].uuid(), Some(uuid));
        assert_eq!(parsed.boxes()[0].payload(), b"raw");
    }

    #[test]
    fn child_iteration_and_path_lookup_are_bounded() {
        let child = box_bytes(b"trak", b"");
        let moov = box_bytes(b"moov", &child);
        let mut source = box_bytes(b"ftyp", b"crx ");
        source.extend_from_slice(&moov);

        let bmff = Bmff::parse(&source).unwrap();
        let moov = bmff.find(&[FourCc::new(*b"moov")]).unwrap().unwrap();
        let children = moov.children(BmffLimits::default()).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].typ(), FourCc::new(*b"trak"));
        assert!(
            bmff.find(&[FourCc::new(*b"moov"), FourCc::new(*b"trak")])
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn rejects_truncated_header_and_payload() {
        assert!(matches!(
            Bmff::parse(&[0, 0, 0]),
            Err(BmffError::InvalidInput(_))
        ));
        let mut source = 16_u32.to_be_bytes().to_vec();
        source.extend_from_slice(b"moov");
        source.extend_from_slice(&[0; 4]);
        assert!(matches!(
            Bmff::parse(&source),
            Err(BmffError::InvalidInput(_))
        ));
    }

    #[test]
    fn rejects_oversized_boxes_and_excessive_depth() {
        let limits = BmffLimits {
            max_box_size: 12,
            ..BmffLimits::default()
        };
        let source = box_bytes(b"free", &[0; 8]);
        assert!(matches!(
            Bmff::parse_with_limits(&source, limits),
            Err(BmffError::LimitExceeded(_))
        ));

        let source = box_bytes(b"moov", &box_bytes(b"trak", b""));
        let bmff = Bmff::parse(&source).unwrap();
        let limits = BmffLimits {
            max_depth: 0,
            ..BmffLimits::default()
        };
        assert!(matches!(
            bmff.find_with_limits(&[FourCc::new(*b"moov"), FourCc::new(*b"trak")], &limits),
            Err(BmffError::LimitExceeded(_))
        ));
    }

    #[test]
    fn truncation_and_byte_flip_sweeps_never_panic() {
        let source = box_bytes(b"moov", &box_bytes(b"trak", &box_bytes(b"mdia", b"")));
        for length in 0..source.len() {
            let result = std::panic::catch_unwind(|| {
                Bmff::parse(&source[..length]).map(|parsed| parsed.boxes().len())
            });
            assert!(
                result.is_ok(),
                "parser panicked at truncation length {length}"
            );
        }
        for index in 0..source.len() {
            let mut mutated = source.clone();
            mutated[index] ^= 0x80;
            let result = std::panic::catch_unwind(|| Bmff::parse(&mutated));
            assert!(
                result.is_ok(),
                "parser panicked after flipping byte {index}"
            );
        }
    }
}
