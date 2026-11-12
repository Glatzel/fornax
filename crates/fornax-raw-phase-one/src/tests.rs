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
