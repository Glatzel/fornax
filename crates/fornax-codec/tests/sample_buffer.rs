use fornax_codec::SampleBuffer;

#[test]
fn constructors_preserve_dimensions_for_every_sample_type() {
    let buffers = [
        SampleBuffer::from_u8(2, 1, vec![1, 2]).unwrap(),
        SampleBuffer::from_u16(2, 1, vec![1, 2]).unwrap(),
        SampleBuffer::from_u32(2, 1, vec![1, 2]).unwrap(),
        SampleBuffer::from_f32(2, 1, vec![1.0, 2.0]).unwrap(),
    ];

    for buffer in buffers {
        assert_eq!((buffer.width(), buffer.height()), (2, 1));
    }
}

#[test]
fn constructors_reject_short_and_long_sample_buffers() {
    assert!(SampleBuffer::from_u8(2, 2, vec![1, 2, 3]).is_err());
    assert!(SampleBuffer::from_u8(2, 2, vec![1, 2, 3, 4, 5]).is_err());
    assert!(SampleBuffer::from_u16(2, 2, vec![1, 2, 3]).is_err());
    assert!(SampleBuffer::from_u16(2, 2, vec![1, 2, 3, 4, 5]).is_err());
    assert!(SampleBuffer::from_u32(2, 2, vec![1, 2, 3]).is_err());
    assert!(SampleBuffer::from_u32(2, 2, vec![1, 2, 3, 4, 5]).is_err());
    assert!(SampleBuffer::from_f32(2, 2, vec![1.0, 2.0, 3.0]).is_err());
    assert!(SampleBuffer::from_f32(2, 2, vec![1.0, 2.0, 3.0, 4.0, 5.0]).is_err());
}

#[test]
fn raw_extraction_preserves_samples_and_allocation_for_every_type() {
    let u8_data = vec![1, 2, 3, 4];
    let u8_pointer = u8_data.as_ptr();
    assert_eq!(
        SampleBuffer::from_u8(2, 2, u8_data)
            .unwrap()
            .into_raw_u8()
            .unwrap(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        SampleBuffer::from_u8(2, 2, vec![1, 2, 3, 4])
            .unwrap()
            .into_raw_u8()
            .unwrap()
            .as_ptr(),
        u8_pointer
    );

    let u16_data = vec![1, 2, 3, 4];
    let u16_pointer = u16_data.as_ptr();
    assert_eq!(
        SampleBuffer::from_u16(2, 2, u16_data)
            .unwrap()
            .into_raw_u16()
            .unwrap(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        SampleBuffer::from_u16(2, 2, vec![1, 2, 3, 4])
            .unwrap()
            .into_raw_u16()
            .unwrap()
            .as_ptr(),
        u16_pointer
    );

    let u32_data = vec![1, 2, 3, 4];
    let u32_pointer = u32_data.as_ptr();
    assert_eq!(
        SampleBuffer::from_u32(2, 2, u32_data)
            .unwrap()
            .into_raw_u32()
            .unwrap(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        SampleBuffer::from_u32(2, 2, vec![1, 2, 3, 4])
            .unwrap()
            .into_raw_u32()
            .unwrap()
            .as_ptr(),
        u32_pointer
    );

    let f32_data = vec![1.0, -0.0, f32::from_bits(0x7fc0_1234), 4.0];
    let f32_pointer = f32_data.as_ptr();
    let extracted = SampleBuffer::from_f32(2, 2, f32_data)
        .unwrap()
        .into_raw_f32()
        .unwrap();
    assert_eq!(
        extracted.iter().map(|value| value.to_bits()).collect::<Vec<_>>(),
        [1.0_f32, -0.0, f32::from_bits(0x7fc0_1234), 4.0]
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(extracted.as_ptr(), f32_pointer);
}

#[test]
fn raw_extraction_returns_none_for_a_different_sample_type() {
    assert!(SampleBuffer::from_u8(1, 1, vec![1])
        .unwrap()
        .into_raw_u16()
        .is_none());
    assert!(SampleBuffer::from_u16(1, 1, vec![1])
        .unwrap()
        .into_raw_u32()
        .is_none());
    assert!(SampleBuffer::from_u32(1, 1, vec![1])
        .unwrap()
        .into_raw_f32()
        .is_none());
    assert!(SampleBuffer::from_f32(1, 1, vec![1.0])
        .unwrap()
        .into_raw_u8()
        .is_none());
}
