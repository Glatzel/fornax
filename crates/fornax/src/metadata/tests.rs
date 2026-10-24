use std::num::NonZeroU32;

use super::*;
use crate::SampleFormat;

fn provenance(authority: AuthorityClass) -> Provenance {
    Provenance::new(
        ProviderId::try_new("test").unwrap(),
        SourceKind::Derived {
            operation: "fixture".into(),
        },
        authority,
        Confidence::Exact,
    )
}

fn decoder_provenance() -> DecoderProvenance {
    DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None)
}

#[test]
fn metadata_keys_follow_the_canonical_grammar() {
    for valid in [
        "dng.tag_50740",
        "vendor.sony.white_balance_note",
        "capture.as_shot",
    ] {
        assert!(MetadataKey::try_new(valid).is_ok(), "{valid}");
    }
    for invalid in [
        "",
        "key",
        "unknown.tag",
        "Dng.tag",
        "dng..tag",
        "dng.Tag",
        "dng.tag.value!",
    ] {
        assert!(MetadataKey::try_new(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn extensible_metadata_is_ordered_and_rejects_duplicates() {
    let entry = |key: &str| MetadataEntry {
        key: MetadataKey::try_new(key).unwrap(),
        value: MetadataValue::U64(1),
        unit: None,
        coordinate_space: None,
        provenance: provenance(AuthorityClass::DecoderObservation),
        original: None,
    };
    let mut metadata = ExtensibleMetadata::default();
    metadata.insert(entry("vendor.z")).unwrap();
    metadata.insert(entry("dng.a")).unwrap();
    assert_eq!(
        metadata
            .entries()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["dng.a", "vendor.z"]
    );
    assert!(metadata.insert(entry("vendor.z")).is_err());
}

#[test]
fn typed_concepts_cannot_be_redefined_in_the_extension_map() {
    let mut metadata = ExtensibleMetadata::default();
    let entry = MetadataEntry {
        key: MetadataKey::try_new("raw.black").unwrap(),
        value: MetadataValue::U64(0),
        unit: None,
        coordinate_space: None,
        provenance: provenance(AuthorityClass::UserOverride),
        original: None,
    };
    assert!(matches!(
        metadata.insert(entry),
        Err(crate::MetadataError::DuplicateTypedField(_))
    ));
}

#[test]
fn same_authority_candidates_conflict_but_higher_authority_wins() {
    let field = MetadataField::Typed(TypedMetadataField::Black);
    assert_eq!(
        resolve_sourced(
            field.clone(),
            Sourced::new(1, provenance(AuthorityClass::DecoderObservation)),
            Sourced::new(2, provenance(AuthorityClass::DecoderObservation)),
        ),
        Err(crate::MetadataError::Conflict(field.clone()))
    );
    assert_eq!(
        resolve_sourced(
            field,
            Sourced::new(1, provenance(AuthorityClass::DecoderObservation)),
            Sourced::new(2, provenance(AuthorityClass::FormatDefault)),
        )
        .unwrap()
        .value,
        1
    );
}

#[test]
fn levels_enforce_shape_and_value_invariants() {
    let one = NonZeroU32::new(1).unwrap();
    let black = Rational::new(64, one);
    assert!(BlackLevels::try_new(black, [black; 4], None).is_ok());
    assert!(BlackTile::try_new(one, NonZeroU32::new(2).unwrap(), one, vec![black]).is_err());
    assert!(WhiteLevels::try_new(vec![black], None, None).is_ok());
    assert!(WhiteLevels::try_new(vec![black; 2], None, None).is_err());
}

#[test]
fn validated_metadata_enforces_required_and_cross_field_invariants() {
    use std::num::NonZeroU64;

    let dimensions = PixelSize::new(8, 8).unwrap();
    let active_area = ActiveArea::try_new(dimensions, 1, 2, 8, 8).unwrap();
    let different_area = ActiveArea::try_new(dimensions, 0, 0, 8, 8).unwrap();
    let geometry = StoredGeometry::new(dimensions, NonZeroU64::new(16).unwrap());
    let spec = SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap();
    let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
    let white = Rational::new(65_535, NonZeroU32::new(1).unwrap());
    let common = CommonMetadata {
        sample_spec: Some(Sourced::new(
            spec,
            provenance(AuthorityClass::DecoderObservation),
        )),
        stored_geometry: Some(Sourced::new(
            geometry,
            provenance(AuthorityClass::DecoderObservation),
        )),
        active_area: Some(Sourced::new(
            active_area,
            provenance(AuthorityClass::DecoderObservation),
        )),
        cfa: Some(Sourced::new(
            BayerLayout::new(BayerPattern::Rggb, different_area),
            provenance(AuthorityClass::DecoderObservation),
        )),
        black: Some(Sourced::new(
            BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
            provenance(AuthorityClass::DecoderObservation),
        )),
        white: Some(Sourced::new(
            WhiteLevels::try_new(vec![white], None, None).unwrap(),
            provenance(AuthorityClass::DecoderObservation),
        )),
        ..CommonMetadata::default()
    };
    let missing_black = CommonMetadata {
        black: None,
        ..common.clone()
    };
    assert_eq!(
        ValidatedMetadata::try_new(
            missing_black,
            ExtensibleMetadata::default(),
            decoder_provenance()
        ),
        Err(RawValidationError::Metadata(
            crate::MetadataError::MissingRequiredField(TypedMetadataField::Black)
        ))
    );
    let missing_white = CommonMetadata {
        white: None,
        ..common.clone()
    };
    assert_eq!(
        ValidatedMetadata::try_new(
            missing_white,
            ExtensibleMetadata::default(),
            decoder_provenance()
        ),
        Err(RawValidationError::Metadata(
            crate::MetadataError::MissingRequiredField(TypedMetadataField::White)
        ))
    );
    assert_eq!(
        ValidatedMetadata::try_new(common, ExtensibleMetadata::default(), decoder_provenance()),
        Err(RawValidationError::InvalidCfa)
    );
    assert_eq!(
        ValidatedMetadata::try_new(
            CommonMetadata::default(),
            ExtensibleMetadata::default(),
            decoder_provenance()
        ),
        Err(RawValidationError::Metadata(
            crate::MetadataError::MissingRequiredField(TypedMetadataField::SampleSpec)
        ))
    );
}

#[test]
fn validated_metadata_rejects_invalid_black_white_range() {
    use std::num::NonZeroU64;

    let dimensions = PixelSize::new(2, 2).unwrap();
    let active_area = ActiveArea::try_new(dimensions, 0, 0, 2, 2).unwrap();
    let black = Rational::new(100, NonZeroU32::new(1).unwrap());
    let white = Rational::new(50, NonZeroU32::new(1).unwrap());
    let common = CommonMetadata {
        sample_spec: Some(Sourced::new(
            SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap(),
            provenance(AuthorityClass::DecoderObservation),
        )),
        stored_geometry: Some(Sourced::new(
            StoredGeometry::new(dimensions, NonZeroU64::new(4).unwrap()),
            provenance(AuthorityClass::DecoderObservation),
        )),
        active_area: Some(Sourced::new(
            active_area,
            provenance(AuthorityClass::DecoderObservation),
        )),
        cfa: Some(Sourced::new(
            BayerLayout::new(BayerPattern::Rggb, active_area),
            provenance(AuthorityClass::DecoderObservation),
        )),
        black: Some(Sourced::new(
            BlackLevels::try_new(black, [black; 4], None).unwrap(),
            provenance(AuthorityClass::DecoderObservation),
        )),
        white: Some(Sourced::new(
            WhiteLevels::try_new(vec![white], None, None).unwrap(),
            provenance(AuthorityClass::DecoderObservation),
        )),
        ..CommonMetadata::default()
    };
    assert_eq!(
        ValidatedMetadata::try_new(common, ExtensibleMetadata::default(), decoder_provenance()),
        Err(RawValidationError::Metadata(
            crate::MetadataError::InvalidLevels
        ))
    );
}
