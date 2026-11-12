use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Directory, Tag, Value};

use crate::invalid;

#[derive(Clone, Debug)]
pub struct Profile {
    pub make: &'static str,
    pub model: &'static str,
    pub cfa: [u8; 4],
}

const PROFILES: &[Profile] = &[
    Profile {
        make: "Leaf",
        model: "Credo 40",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "H 25",
        cfa: [1, 2, 0, 1],
    },
    Profile {
        make: "Phase One",
        model: "IQ140",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "IQ150",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "IQ180",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "IQ3 100MP",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "IQ3 100MP Trichr",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "IQ4 150MP",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "P20+",
        cfa: [1, 0, 2, 1],
    },
    Profile {
        make: "Phase One",
        model: "P25",
        cfa: [1, 2, 0, 1],
    },
    Profile {
        make: "Phase One",
        model: "P25+",
        cfa: [1, 2, 0, 1],
    },
    Profile {
        make: "Phase One",
        model: "P30",
        cfa: [1, 0, 2, 1],
    },
    Profile {
        make: "Phase One",
        model: "P45",
        cfa: [1, 0, 2, 1],
    },
    Profile {
        make: "Phase One",
        model: "P45+",
        cfa: [1, 0, 2, 1],
    },
    Profile {
        make: "Phase One",
        model: "P65",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "P65+",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "iXM-100",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "iXM-RS150F",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "iXU180",
        cfa: [0, 1, 1, 2],
    },
    Profile {
        make: "Phase One",
        model: "iXU-RS1000 NIR",
        cfa: [0, 1, 1, 2],
    },
];

pub fn resolve(root: &Directory) -> Result<Profile, DecodeError> {
    let make = ascii(root, TiffTag::Make).ok_or_else(|| invalid("IIQ Make is missing"))?;
    let model = ascii(root, TiffTag::Model).ok_or_else(|| invalid("IIQ Model is missing"))?;
    let make_family = if make.eq_ignore_ascii_case("Leaf") {
        "Leaf"
    } else if make.to_ascii_lowercase().starts_with("phase one") {
        "Phase One"
    } else {
        return Err(DecodeError::Unsupported(
            format!("IIQ camera make {make:?} is unsupported").into(),
        ));
    };
    PROFILES
        .iter()
        .find(|profile| {
            profile.make == make_family && profile.model.eq_ignore_ascii_case(model.trim())
        })
        .cloned()
        .ok_or_else(|| {
            DecodeError::Unsupported(
                format!("IIQ camera model {make_family} {model:?} is unsupported").into(),
            )
        })
}

fn ascii(directory: &Directory, tag: TiffTag) -> Option<String> {
    match directory.get(&Tag::from(tag))? {
        Value::Ascii(value) => Some(value.trim_matches('\0').trim().to_owned()),
        _ => None,
    }
}
