use core::slice;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use fornax::Fornax;

fn main() {
    let mut processor = Fornax::new();
    processor
        .open_file(PathBuf::from(
            "./external/raw-images/images/colorchart-5D2-6000K.dng",
        ))
        .unwrap();
    processor.unpack().unwrap();
    let processed = processor.dcraw_process(None).unwrap();
    println!(
        "{},{},{},{},{},{}",
        processed.width(),
        processed.height(),
        processed.bits(),
        processed.image_type().unwrap(),
        processed.colors(),
        processed.data_size()
    );

    let img: image::ImageBuffer<image::Rgb<u8>, &[u8]> = image::ImageBuffer::from_raw(
        processed.width() as u32,
        processed.height() as u32,
        unsafe { slice::from_raw_parts(processed.data(), processed.data_size() as usize) },
    )
    .unwrap();
    img.save("test.tiff").unwrap();
}
