pub trait PresetCg {
    fn preset_cg(&mut self) -> &mut Self;
}
impl PresetCg for fornax::OutputParams {
    fn preset_cg(&mut self) -> &mut Self {
        self.gamm = Some([1.0, 1.0]);
        self.output_color = Some(fornax::OutputColor::ACES);
        self.output_bps = Some(fornax::OutputBps::_16bit);
        self
    }
}
