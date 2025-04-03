use core::slice;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use fornax::Fornax;

fn main() {
    let mut processor = Fornax::new();
    processor
        .open_file(PathBuf::from(
            "./external/raw-images/images/colorchart-eos-7d.cr2",
        ))
        .unwrap();

    let processed = processor.dcraw_process(None).unwrap();

    let mut out = File::create("out_16bit.ppm").expect("create out");
    let header = format!(
        "P6 {} {} {}\n",
        processed.width(),
        processed.height(),
        255
    );
    out.write_all(header.as_ref()).expect("header");
    // PPM files must be in big endian
    let mut out_vec = Vec::with_capacity(processed.data_size() as usize * 2);
    for chunk in
        unsafe { slice::from_raw_parts(processed.data(), processed.data_size() as usize).iter() }
    {
        out_vec.extend_from_slice(&chunk.to_be_bytes());
    }
    out.write_all(&out_vec).expect("pixels");
}
