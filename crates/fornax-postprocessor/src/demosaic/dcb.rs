use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::DenseLazy;
use super::directional::{self, Profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicDcb;

impl DemosaicDcb {
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        directional::lazy(image, pattern, Profile::Dcb)
    }
}
