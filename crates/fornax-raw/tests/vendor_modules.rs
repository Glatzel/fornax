use fornax_raw::canon::Cr3Image;
use fornax_raw::fujifilm::RafImage;
use fornax_raw::hasselblad::HasselbladImage;
use fornax_raw::nikon::NefImage;
use fornax_raw::panasonic::Rw2Image;
use fornax_raw::phase_one::IiqImage;
use fornax_raw::sony::ArwImage;

#[test]
fn every_vendor_decoder_is_an_unconditional_public_module() {
    let _ = std::any::TypeId::of::<Cr3Image>();
    let _ = std::any::TypeId::of::<RafImage>();
    let _ = std::any::TypeId::of::<HasselbladImage>();
    let _ = std::any::TypeId::of::<NefImage>();
    let _ = std::any::TypeId::of::<Rw2Image>();
    let _ = std::any::TypeId::of::<IiqImage>();
    let _ = std::any::TypeId::of::<ArwImage>();
}
