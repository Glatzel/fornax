impl crate::OutputParams {
    /// Match output to cg workflow.
    /// - `gamm` = `[1.0, 1.0]`
    /// - `output_color`: ACES
    /// - `output_bps`: 16bit
    pub fn preset_cg(&mut self) -> &mut Self {
        self.gamm = Some([1.0, 1.0]);
        self.output_color = Some(crate::OutputColor::ACES);
        self.output_bps = Some(crate::OutputBps::_16bit);
        self
    }
}
