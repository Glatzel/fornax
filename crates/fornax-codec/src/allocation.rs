use fornax_large_alloc::{AllocError, zeroed_samples as allocate_zeroed_samples};

use crate::CodecError;
use crate::error::invalid;

pub(crate) fn zeroed_samples<T>(len: usize) -> Result<Vec<T>, CodecError>
where
    T: bytemuck::Pod,
{
    allocate_zeroed_samples(len).map_err(map_alloc_error)
}

fn map_alloc_error(error: AllocError) -> CodecError {
    invalid(format!("sample allocation layout overflow: {error}"))
}
