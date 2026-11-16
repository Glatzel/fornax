#[test]
fn final_decode_outputs_do_not_reintroduce_direct_sample_vecs() {
    let guarded_sources = [
        (
            "samples.rs",
            include_str!("../src/samples.rs"),
            &[
                "let mut output = vec![0_u8; sample_count]",
                "let mut output = vec![0_u16; sample_count]",
                "let mut output = vec![0_u32; sample_count]",
                "let mut output = vec![0_f32; sample_count]",
                "let mut output = vec![0_f64; sample_count]",
            ][..],
        ),
        (
            "deflate.rs",
            include_str!("../src/deflate.rs"),
            &["let mut output = vec![0.0_f32; sample_count]"][..],
        ),
        (
            "ljpeg.rs",
            include_str!("../src/ljpeg.rs"),
            &["let mut output = vec![0_u16; sample_count]"][..],
        ),
        (
            "jpeg.rs",
            include_str!("../src/jpeg.rs"),
            &["let mut output = vec![0_u8; sample_count]"][..],
        ),
        (
            "jpegxl.rs",
            include_str!("../src/jpegxl.rs"),
            &[
                "let mut output = vec![0_u8; sample_count]",
                "let mut output = vec![0_u16; sample_count]",
            ][..],
        ),
    ];

    for (file, source, forbidden_patterns) in guarded_sources {
        for pattern in forbidden_patterns {
            assert!(
                !source.contains(pattern),
                "{file} contains a final-output allocation outside `zeroed_samples`: {pattern}"
            );
        }
    }
}
