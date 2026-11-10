use fornax_tiff::{Directory, Tag, Value, ValueAccessError, unsigned_scalar, unsigned_vec};

#[test]
fn unsigned_scalar_accepts_tiff_unsigned_variants() {
    for (value, expected) in [
        (Value::Byte(1), 1),
        (Value::Short(2), 2),
        (Value::Unsigned(3), 3),
        (Value::Ifd(4), 4),
        (Value::UnsignedBig(5), 5),
        (Value::IfdBig(6), 6),
    ] {
        assert_eq!(unsigned_scalar(Some(&value)), Some(expected), "{value:?}");
    }
    assert_eq!(unsigned_scalar(None), None);
}

#[test]
fn unsigned_scalar_recurses_into_the_first_list_value() {
    let value = Value::List(vec![
        Value::List(vec![Value::Unsigned(7), Value::Unsigned(8)]),
        Value::Unsigned(9),
    ]);

    assert_eq!(unsigned_scalar(Some(&value)), Some(7));
    assert_eq!(unsigned_scalar(Some(&Value::List(Vec::new()))), None);
}

#[test]
fn unsigned_scalar_rejects_non_unsigned_variants() {
    for value in [
        Value::SignedByte(1),
        Value::SignedShort(1),
        Value::Signed(1),
        Value::SignedBig(1),
        Value::Float(1.0),
        Value::Double(1.0),
        Value::Rational(1, 1),
        Value::SRational(1, 1),
        Value::Ascii("1".into()),
    ] {
        assert_eq!(unsigned_scalar(Some(&value)), None, "{value:?}");
    }
    assert_eq!(
        unsigned_scalar(Some(&Value::List(vec![Value::Signed(1)]))),
        None
    );
}

#[test]
fn unsigned_vec_converts_scalars_and_flat_lists_in_order() {
    let mut directory = Directory::default();

    directory.insert(Tag(1), Value::Unsigned(7));
    assert_eq!(unsigned_vec::<_, u64>(&directory, Tag(1)).unwrap(), vec![7]);

    directory.insert(
        Tag(2),
        Value::List(vec![
            Value::Byte(1),
            Value::Short(2),
            Value::Signed(3),
            Value::UnsignedBig(4),
            Value::Ifd(5),
            Value::IfdBig(6),
        ]),
    );
    assert_eq!(
        unsigned_vec::<_, u64>(&directory, Tag(2)).unwrap(),
        vec![1, 2, 3, 4, 5, 6]
    );

    directory.insert(Tag(3), Value::List(Vec::new()));
    assert_eq!(
        unsigned_vec::<_, u64>(&directory, Tag(3)).unwrap(),
        Vec::<u64>::new()
    );
}

#[test]
fn unsigned_vec_reports_missing_and_rejected_values() {
    let mut directory = Directory::default();

    assert_eq!(
        unsigned_vec::<_, u64>(&directory, Tag(1)).unwrap_err(),
        ValueAccessError::Missing
    );

    directory.insert(Tag(1), Value::Signed(-1));
    assert_eq!(
        unsigned_vec::<_, u64>(&directory, Tag(1)).unwrap_err(),
        ValueAccessError::InvalidType
    );

    directory.insert(Tag(1), Value::Rational(1, 1));
    assert_eq!(
        unsigned_vec::<_, u64>(&directory, Tag(1)).unwrap_err(),
        ValueAccessError::InvalidType
    );

    directory.insert(
        Tag(1),
        Value::List(vec![Value::List(vec![Value::Unsigned(1)])]),
    );
    assert_eq!(
        unsigned_vec::<_, u64>(&directory, Tag(1)).unwrap_err(),
        ValueAccessError::InvalidType
    );
}

#[test]
fn unsigned_vec_reports_target_width_overflow() {
    let mut directory = Directory::default();
    directory.insert(Tag(1), Value::Unsigned(256));

    assert_eq!(
        unsigned_vec::<_, u8>(&directory, Tag(1)).unwrap_err(),
        ValueAccessError::Overflow
    );
}
