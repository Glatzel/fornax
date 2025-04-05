pub enum Preview {
    None,
    Medium,
    Full,
}
pub enum Compatibility {
    CR2_4,
    CR4_1,
    CR4_6,
    CR5_4,
    CR6_6,
    CR7_1,
    CR11_2,
    CR12_4,
    CR13_2,
    CR14_0,
    CR15_3,
    CR16_0,

    DNG1_1,
    DNG1_3,
    DNG1_4,
    DNG1_5,
    DNG1_6,
    DNG1_7,
    DNG1_7_1,
}
pub struct DngConverterParams {
    ///Output lossless compressed DNG files
    pub compressed: bool,
    /// Output linear DNG files.
    pub linear: bool,
    ///Embed original raw file inside DNG files.
    pub embed: bool,
    ///Set JPEG preview size.
    pub preview: Preview,
    ///Embed fast load data inside DNG files.
    pub fast_load: bool,
    ///Limit size to <num> pixels/side.
    pub side: Option<u32>,
    ///Limit pixel count to <num> pixels/image.
    pub count: Option<u32>,
    ///Limit pixel count to <num> pixels/image.
    pub compatibility: Compatibility,
}
impl Default for DngConverterParams {
    fn default() -> Self {
        Self {
            compressed: true,
            linear: false,
            embed: false,
            preview: Preview::Medium,
            fast_load: false,
            side: None,
            count: None,
            compatibility: Compatibility::CR16_0,
        }
    }
}
