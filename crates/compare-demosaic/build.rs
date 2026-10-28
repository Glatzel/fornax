use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

const LIBRAW_SOURCES: &[&str] = &[
    "src/decoders/canon_600.cpp",
    "src/decoders/crx.cpp",
    "src/decoders/decoders_dcraw.cpp",
    "src/decoders/decoders_libraw.cpp",
    "src/decoders/decoders_libraw_dcrdefs.cpp",
    "src/decoders/dng.cpp",
    "src/decoders/fp_dng.cpp",
    "src/decoders/fuji_compressed.cpp",
    "src/decoders/generic.cpp",
    "src/decoders/kodak_decoders.cpp",
    "src/decoders/load_mfbacks.cpp",
    "src/decoders/olympus14.cpp",
    "src/decoders/pana8.cpp",
    "src/decoders/smal.cpp",
    "src/decoders/sonycc.cpp",
    "src/decoders/unpack.cpp",
    "src/decoders/unpack_thumb.cpp",
    "src/decompressors/losslessjpeg.cpp",
    "src/demosaic/aahd_demosaic.cpp",
    "src/demosaic/ahd_demosaic.cpp",
    "src/demosaic/dcb_demosaic.cpp",
    "src/demosaic/dht_demosaic.cpp",
    "src/demosaic/misc_demosaic.cpp",
    "src/demosaic/xtrans_demosaic.cpp",
    "src/integration/dngsdk_glue.cpp",
    "src/integration/rawspeed_glue.cpp",
    "src/libraw_c_api.cpp",
    "src/libraw_datastream.cpp",
    "src/metadata/adobepano.cpp",
    "src/metadata/canon.cpp",
    "src/metadata/ciff.cpp",
    "src/metadata/cr3_parser.cpp",
    "src/metadata/epson.cpp",
    "src/metadata/exif_gps.cpp",
    "src/metadata/fuji.cpp",
    "src/metadata/hasselblad_model.cpp",
    "src/metadata/identify.cpp",
    "src/metadata/identify_tools.cpp",
    "src/metadata/kodak.cpp",
    "src/metadata/leica.cpp",
    "src/metadata/makernotes.cpp",
    "src/metadata/mediumformat.cpp",
    "src/metadata/minolta.cpp",
    "src/metadata/misc_parsers.cpp",
    "src/metadata/nikon.cpp",
    "src/metadata/normalize_model.cpp",
    "src/metadata/olympus.cpp",
    "src/metadata/p1.cpp",
    "src/metadata/pentax.cpp",
    "src/metadata/samsung.cpp",
    "src/metadata/sony.cpp",
    "src/metadata/tiff.cpp",
    "src/postprocessing/aspect_ratio.cpp",
    "src/postprocessing/dcraw_process.cpp",
    "src/postprocessing/mem_image.cpp",
    "src/postprocessing/postprocessing_aux.cpp",
    "src/postprocessing/postprocessing_utils.cpp",
    "src/postprocessing/postprocessing_utils_dcrdefs.cpp",
    "src/preprocessing/ext_preprocess.cpp",
    "src/preprocessing/raw2image.cpp",
    "src/preprocessing/subtract_black.cpp",
    "src/tables/cameralist.cpp",
    "src/tables/colorconst.cpp",
    "src/tables/colordata.cpp",
    "src/tables/wblists.cpp",
    "src/utils/curves.cpp",
    "src/utils/decoder_info.cpp",
    "src/utils/init_close_utils.cpp",
    "src/utils/open.cpp",
    "src/utils/phaseone_processing.cpp",
    "src/utils/read_utils.cpp",
    "src/utils/thumb_utils.cpp",
    "src/utils/utils_dcraw.cpp",
    "src/utils/utils_libraw.cpp",
    "src/write/apply_profile.cpp",
    "src/write/file_write.cpp",
    "src/write/tiff_writer.cpp",
    "src/x3f/x3f_parse_process.cpp",
    "src/x3f/x3f_utils_patched.cpp",
];

const SHIM_SOURCES: &[&str] = &["cpp/algos/linear.cpp", "cpp/shim.cpp"];

struct SourcePatch {
    source: &'static str,
    replacements: &'static [(&'static str, &'static str)],
}

// Keep empty unless a reference routine is inaccessible in unmodified LibRaw.
// Every replacement must match exactly once or the build fails.
const SOURCE_PATCHES: &[SourcePatch] = &[];

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .ancestors()
        .nth(2)
        .expect("compare-demosaic must be inside the workspace")
        .to_path_buf();
    let libraw_root = workspace_root.join("external/libraw");

    if !libraw_root.join("src").is_dir() {
        panic!(
            "LibRaw source is missing at {}. Run `git submodule update --init external/libraw`.",
            libraw_root.display()
        );
    }

    println!("cargo:rerun-if-changed=cpp");
    println!(
        "cargo:rerun-if-changed={}",
        libraw_root.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        libraw_root.join("internal").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        libraw_root.join("libraw").display()
    );

    if let Some(commit) = git_commit(&libraw_root) {
        println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_COMMIT={commit}");
    } else {
        println!("cargo:warning=LibRaw commit could not be read from git");
        println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_COMMIT=unknown");
    }
    let version = libraw_version(&libraw_root);
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_VERSION={version}");
    write_status_codes(&manifest_dir);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .warnings(false)
        .include(&libraw_root)
        .define("LIBRAW_NOTHREADS", None)
        .define("NODEPS", None)
        .flag_if_supported("-std=c++17");

    for source in LIBRAW_SOURCES {
        let original = libraw_root.join(source);
        assert!(
            original.is_file(),
            "missing LibRaw source: {}",
            original.display()
        );
        let patch = SOURCE_PATCHES.iter().find(|patch| patch.source == *source);
        let path = if let Some(patch) = patch {
            patched_source(&original, patch).expect("applying LibRaw source replacement")
        } else {
            original
        };
        build.file(path);
    }
    for source in SHIM_SOURCES {
        build.file(manifest_dir.join(source));
    }
    build.compile("compare_demosaic_libraw_reference");
}

fn write_status_codes(manifest_dir: &Path) {
    let header =
        fs::read_to_string(manifest_dir.join("cpp/shim.h")).expect("reading comparison FFI header");
    let mut generated = String::from("// Generated from cpp/shim.h by build.rs.\n");
    let mut count = 0;
    for line in header
        .lines()
        .filter_map(|line| line.trim().strip_prefix("#define COMPARE_DEMOSAIC_STATUS_"))
    {
        let (name, value) = line
            .split_once(' ')
            .expect("comparison status macro must contain a value");
        let name = format!("COMPARE_DEMOSAIC_STATUS_{name}");
        value
            .parse::<i32>()
            .unwrap_or_else(|_| panic!("invalid value for {name}"));
        generated.push_str(&format!("pub const {name}: i32 = {value};\n"));
        count += 1;
    }
    assert_eq!(count, 8, "comparison FFI header must define 8 statuses");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("status_codes.rs");
    fs::write(output, generated).expect("writing generated comparison FFI statuses");
}

fn patched_source(original: &Path, patch: &SourcePatch) -> std::io::Result<PathBuf> {
    let mut source = fs::read_to_string(original)?;
    for (old, new) in patch.replacements {
        let matches = source.matches(old).count();
        assert_eq!(
            matches,
            1,
            "LibRaw patch target in {} matched {matches} times: {old:?}",
            original.display()
        );
        source = source.replacen(old, new, 1);
    }
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::create_dir_all(&output_dir)?;
    let filename = patch.source.replace(['/', '\\'], "_");
    let output = output_dir.join(filename);
    fs::write(&output, source)?;
    Ok(output)
}

fn git_commit(libraw_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(libraw_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(commit)
}

fn libraw_version(libraw_root: &Path) -> String {
    let header = fs::read_to_string(libraw_root.join("libraw/libraw_version.h"))
        .expect("reading LibRaw version header");
    let field = |name: &str| {
        header
            .lines()
            .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
            .unwrap_or_else(|| panic!("missing {name} in LibRaw version header"))
            .trim()
    };
    format!(
        "{}.{}.{}",
        field("LIBRAW_MAJOR_VERSION"),
        field("LIBRAW_MINOR_VERSION"),
        field("LIBRAW_PATCH_VERSION")
    )
}
