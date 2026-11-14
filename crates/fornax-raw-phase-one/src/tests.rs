use fornax_raw_core::EncodingSampleFormat;

use super::profiles;

#[test]
fn phase_one_make_families_resolve_to_the_same_profile() {
    let root = |make: &str| {
        let mut directory = fornax_tiff::Directory::default();
        directory.insert(
            fornax_tiff::Tag::from(fornax_tiff::constants::TiffTag::Make),
            fornax_tiff::Value::Ascii(make.into()),
        );
        directory.insert(
            fornax_tiff::Tag::from(fornax_tiff::constants::TiffTag::Model),
            fornax_tiff::Value::Ascii("P65+".into()),
        );
        directory
    };

    let direct = profiles::resolve(&root("Phase One")).unwrap();
    let aliased = profiles::resolve(&root("Phase One A/S")).unwrap();

    assert_eq!(direct.cfa, aliased.cfa);
    assert_eq!(direct.cfa, [0, 1, 1, 2]);
}

#[test]
fn encoding_metadata_distinguishes_phase_one_compression_modes() {
    for (compression, expected_id, expected_name, expected_bits, lossless) in [
        (
            crate::container::Compression::Uncompressed,
            0,
            "iiq-uncompressed",
            14,
            true,
        ),
        (
            crate::container::Compression::Lossless14,
            3,
            "iiq-lossless14",
            14,
            true,
        ),
        (
            crate::container::Compression::Lossless16,
            8,
            "iiq-lossless16",
            16,
            true,
        ),
        (
            crate::container::Compression::Nonlinear,
            5,
            "iiq-nonlinear",
            14,
            false,
        ),
        (crate::container::Compression::Sv2, 6, "iiq-sv2", 14, false),
    ] {
        let layout = crate::container::RawLayout {
            width: 64,
            height: 32,
            compression,
            data_offset: 0,
            data_size: 128,
            strip_offset: None,
            strip_size: None,
        };

        let encoding = crate::encoding_metadata_for_layout(&layout);

        assert_eq!(encoding.compression_id, Some(expected_id));
        assert_eq!(encoding.compression_name, expected_name);
        assert_eq!(encoding.lossless, lossless);
        assert_eq!(
            encoding.storage,
            if compression == crate::container::Compression::Uncompressed {
                "single-chunk"
            } else {
                "iiq-rows"
            }
        );
        assert_eq!(encoding.bits_per_sample, Some(expected_bits));
        assert_eq!(encoding.sample_format, Some(EncodingSampleFormat::Integer));
    }
}
