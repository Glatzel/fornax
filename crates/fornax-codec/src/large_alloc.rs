//! Large zero-initialized sample-buffer allocation.
//!
//! This module returns an owned `Vec<T>` whose elements are all zero. For a
//! final decode output of at least 8 MiB on Linux, it asks the system allocator
//! for already-zeroed memory and advises the page-aligned interior of that live
//! allocation with `MADV_HUGEPAGE` before the decoder writes to it. Every other
//! target, Miri, a build without `large-page-hint`, and smaller outputs use a
//! plain zeroed `Vec`.
//!
//! The policy is host-specific rather than a generic performance claim. On WSL2
//! Debian, AMD Ryzen 7 5800H, kernel `6.6.87.1-microsoft-standard-WSL2`, glibc
//! `2.41-12+deb13u3`, and THP `[madvise]`, PR #730 measured FP16 Deflate at
//! 243.7 -> 181.9 ms at one thread and 56.2 -> 45.9 ms at 15 threads; FP32
//! Deflate was 404.9 -> 338.9 ms and 68.3 -> 62.0 ms. PR #728 measured 5D3
//! LJPEG at 210.5 -> 170.5 ms at one thread and 41.8 -> 38.2 ms at 15 threads.
//! The later portable regression measured 5D3 uncompressed at 17.314 ->
//! 44.383 ms. These numbers come from source commits
//! `2887deb3d669994b7e85d7569803eee3c1a874e6`,
//! `b0606cad16734166a84cd05e4a6f9d4e8a93ecd9`, and
//! `65cbb3815ecce6061bca768a58b43b0890a51aaa`; the implementation is restored
//! from those commits rather than re-derived. FlashRaw 0.2.0
//! (MIT OR Apache-2.0) corroborated the natural-alignment and
//! allocation-before-first-write ordering; no FlashRaw source was copied.
//!
//! The Vec adoption is sound because `alloc_zeroed(Layout::array::<T>(len))`
//! and `Vec<T>` use the same global allocator. The layout has exactly `T`'s
//! natural alignment, `len` elements, and `Vec` is built with `capacity ==
//! len`, so its eventual deallocation uses the same size and alignment. `T:
//! Pod` guarantees that the all-zero bit pattern is valid. Layout overflow is
//! reported before allocation, zero length returns `Vec::new()`, and a null
//! allocation calls `handle_alloc_error`. The pointer is never read before
//! ownership is transferred to the initialized zeroed `Vec`.
//!
//! Natural alignment is essential. An over-aligned `alloc_zeroed` makes Rust
//! zero the allocation before this module can issue `madvise`, silently
//! disabling first-touch huge pages; that ordering was the bug in the first
//! probe.

use bytemuck::Pod;

use crate::CodecError;
use crate::error::invalid;

#[cfg(any(test, all(target_os = "linux", not(miri), feature = "large-page-hint")))]
const HUGE_PAGE_BYTES: usize = 2 * 1024 * 1024;
// Four huge-page spans. Recovered from the measured PR #730 policy: large
// enough for THP to back a useful advisory interior without taxing small
// outputs.
#[cfg(any(test, all(target_os = "linux", not(miri), feature = "large-page-hint")))]
const LARGE_ALLOCATION_THRESHOLD_BYTES: usize = 4 * HUGE_PAGE_BYTES;

pub(super) fn zeroed_samples<T>(len: usize) -> Result<Vec<T>, CodecError>
where
    T: Pod,
{
    let layout = std::alloc::Layout::array::<T>(len)
        .map_err(|error| invalid(format!("sample allocation layout overflow: {error}")))?;
    #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
    if layout.size() >= LARGE_ALLOCATION_THRESHOLD_BYTES {
        #[cfg(test)]
        let previous_policy = tests::enter_policy();
        let output = linux_zeroed_samples(len, layout);
        #[cfg(test)]
        tests::leave_policy(previous_policy);
        return Ok(output);
    }
    let _ = layout;
    portable_zeroed_samples(len)
}

fn portable_zeroed_samples<T>(len: usize) -> Result<Vec<T>, CodecError>
where
    T: Pod,
{
    let layout = std::alloc::Layout::array::<T>(len)
        .map_err(|error| invalid(format!("sample allocation layout overflow: {error}")))?;
    if layout.size() == 0 {
        return Ok(Vec::new());
    }
    Ok(vec![T::zeroed(); len])
}

#[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
fn linux_zeroed_samples<T>(len: usize, layout: std::alloc::Layout) -> Vec<T>
where
    T: Pod,
{
    // SAFETY: `Layout::array::<T>(len)` describes exactly `len` `T` values
    // with T's natural alignment. `alloc_zeroed` returns either null or a live
    // block from the global allocator with that layout; `T: Pod` makes the
    // all-zero contents valid. `Vec::from_raw_parts` adopts that same pointer
    // with the same length and capacity, so its drop uses the same global
    // allocator and layout.
    let mut output = unsafe {
        let pointer = std::alloc::alloc_zeroed(layout).cast::<T>();
        if pointer.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Vec::from_raw_parts(pointer, len, len)
    };
    advise_hugepage(output.as_mut_ptr().cast(), layout);
    output
}

// Advise only the complete 2 MiB-aligned interior of the live allocation.
// Decode has not touched the buffer yet, and an advisory failure merely drops
// the optimization.
#[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
fn advise_hugepage(pointer: *mut u8, layout: std::alloc::Layout) {
    let start_address = pointer as usize;
    let Some(end_address) = start_address.checked_add(layout.size()) else {
        return;
    };
    let Some(interior_start_address) = start_address.checked_add(HUGE_PAGE_BYTES - 1) else {
        return;
    };
    let interior_start = interior_start_address & !(HUGE_PAGE_BYTES - 1);
    let interior_end = end_address & !(HUGE_PAGE_BYTES - 1);
    if interior_end > interior_start {
        // SAFETY: the checked addresses describe a page-aligned, nonempty
        // interior of the allocation that remains live until `output` is
        // dropped. `madvise` changes only kernel backing policy and needs no
        // Rust access to the bytes.
        unsafe {
            libc::madvise(
                interior_start as *mut libc::c_void,
                interior_end - interior_start,
                libc::MADV_HUGEPAGE,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::fmt::Debug;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bytemuck::Pod;

    use super::{LARGE_ALLOCATION_THRESHOLD_BYTES, portable_zeroed_samples, zeroed_samples};

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static TRACKED_SIZE: AtomicUsize = AtomicUsize::new(0);
    static TRACKED_ALIGN: AtomicUsize = AtomicUsize::new(0);
    static TRACKED_POINTER: AtomicUsize = AtomicUsize::new(0);
    static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static POLICY_ZEROED_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static BYPASS_ZEROED_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static DEALLOCATION_LAYOUT_MISMATCHES: AtomicUsize = AtomicUsize::new(0);

    thread_local! {
        static IN_POLICY: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn enter_policy() -> bool { IN_POLICY.replace(true) }

    pub(super) fn leave_policy(previous: bool) { IN_POLICY.set(previous); }

    struct CountingAllocator;

    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let pointer = unsafe { System.alloc(layout) };
            if is_tracked_allocation(layout) && !pointer.is_null() {
                ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
                TRACKED_POINTER.store(pointer as usize, Ordering::SeqCst);
            }
            pointer
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            let pointer = unsafe { System.alloc_zeroed(layout) };
            if is_tracked_allocation(layout) && !pointer.is_null() {
                if IN_POLICY.with(Cell::get) {
                    POLICY_ZEROED_ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
                } else {
                    BYPASS_ZEROED_ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
                }
                TRACKED_POINTER.store(pointer as usize, Ordering::SeqCst);
            }
            pointer
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            if pointer as usize == TRACKED_POINTER.load(Ordering::SeqCst) {
                DEALLOCATIONS.fetch_add(1, Ordering::SeqCst);
                if layout.align() != TRACKED_ALIGN.load(Ordering::SeqCst) {
                    DEALLOCATION_LAYOUT_MISMATCHES.fetch_add(1, Ordering::SeqCst);
                }
                TRACKED_POINTER.store(0, Ordering::SeqCst);
            }
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    fn is_tracked_size(layout: Layout) -> bool {
        let tracked_size = TRACKED_SIZE.load(Ordering::SeqCst);
        tracked_size != 0 && layout.size() == tracked_size
    }

    fn is_tracked_allocation(layout: Layout) -> bool {
        is_tracked_size(layout) && layout.align() == TRACKED_ALIGN.load(Ordering::SeqCst)
    }

    fn reset_tracking(byte_count: usize, alignment: usize) {
        TRACKED_SIZE.store(byte_count, Ordering::SeqCst);
        TRACKED_ALIGN.store(alignment, Ordering::SeqCst);
        TRACKED_POINTER.store(0, Ordering::SeqCst);
        ALLOCATIONS.store(0, Ordering::SeqCst);
        POLICY_ZEROED_ALLOCATIONS.store(0, Ordering::SeqCst);
        BYPASS_ZEROED_ALLOCATIONS.store(0, Ordering::SeqCst);
        DEALLOCATIONS.store(0, Ordering::SeqCst);
        DEALLOCATION_LAYOUT_MISMATCHES.store(0, Ordering::SeqCst);
    }

    fn assert_zeroed_samples_are_fillable<T>(len: usize)
    where
        T: PartialEq + Pod + Debug,
    {
        let mut samples = zeroed_samples::<T>(len).unwrap();
        assert_eq!(samples.len(), len);
        assert_eq!(samples.capacity(), len);
        assert!(samples.iter().all(|sample| *sample == T::zeroed()));

        for sample in &mut samples {
            bytemuck::bytes_of_mut(sample).fill(0xa5);
        }
        drop(samples);
    }

    #[test]
    fn zeroed_samples_support_every_decoded_scalar_with_exact_capacity() {
        assert_zeroed_samples_are_fillable::<u16>(0);
        assert_zeroed_samples_are_fillable::<u16>(
            LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<u16>() - 1,
        );
        assert_zeroed_samples_are_fillable::<u16>(
            LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<u16>() + 1,
        );
        assert_zeroed_samples_are_fillable::<u32>(1);
        assert_zeroed_samples_are_fillable::<f32>(1);
        assert_zeroed_samples_are_fillable::<f64>(1);
    }

    #[test]
    fn zeroed_samples_reject_layout_overflow() {
        assert!(zeroed_samples::<u16>(usize::MAX).is_err());
    }

    fn assert_tracked_allocations<T>(len: usize, expect_large_page_policy: bool)
    where
        T: PartialEq + Pod + Debug,
    {
        let byte_count = len * size_of::<T>();
        reset_tracking(byte_count, align_of::<T>());

        let samples = zeroed_samples::<T>(len).unwrap();
        assert_eq!(samples.len(), len);
        assert_eq!(samples.capacity(), len);

        let allocations = ALLOCATIONS.load(Ordering::SeqCst);
        let bypass_zeroed_allocations = BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst);
        let policy_zeroed_allocations = POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst);
        if byte_count == 0 {
            assert_eq!(
                allocations + bypass_zeroed_allocations + policy_zeroed_allocations,
                0
            );
        } else {
            assert_eq!(
                allocations + bypass_zeroed_allocations + policy_zeroed_allocations,
                1
            );
        }
        #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
        if expect_large_page_policy {
            assert_eq!(allocations, 0);
            assert_eq!(bypass_zeroed_allocations, 0);
            assert_eq!(policy_zeroed_allocations, 1);
        } else {
            assert_eq!(policy_zeroed_allocations, 0);
        }
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(samples);

        if byte_count == 0 {
            assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        }
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[test]
    fn allocation_and_drop_use_matching_natural_alignment_layouts() {
        let _guard = TEST_LOCK.lock().unwrap();
        let sample_count = LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<u16>();

        assert_tracked_allocations::<u16>(0, false);
        assert_tracked_allocations::<u16>(1, false);
        assert_tracked_allocations::<u16>(sample_count - 1, false);
        assert_tracked_allocations::<u16>(sample_count, true);
        assert_tracked_allocations::<u16>(sample_count + 1, true);
        assert_tracked_allocations::<u32>(LARGE_ALLOCATION_THRESHOLD_BYTES / 4 + 1, true);
        assert_tracked_allocations::<f32>(LARGE_ALLOCATION_THRESHOLD_BYTES / 4 + 1, true);
        assert_tracked_allocations::<f64>(LARGE_ALLOCATION_THRESHOLD_BYTES / 8 + 1, true);
    }

    #[test]
    fn forced_portable_fallback_uses_a_normal_zeroed_vec() {
        let _guard = TEST_LOCK.lock().unwrap();
        let len = LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<u16>() + 1;
        let byte_count = len * size_of::<u16>();
        reset_tracking(byte_count, align_of::<u16>());

        let samples = portable_zeroed_samples::<u16>(len).unwrap();
        assert_eq!(samples.len(), len);
        assert_eq!(samples.capacity(), len);
        assert!(samples.iter().all(|sample| *sample == 0));
        assert_eq!(
            ALLOCATIONS.load(Ordering::SeqCst) + BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst),
            1
        );
        assert_eq!(POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(samples);
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[test]
    fn uncompressed_large_output_uses_the_shared_allocation_policy() {
        let _guard = TEST_LOCK.lock().unwrap();
        let len = LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<u16>() + 1;
        let byte_count = len * size_of::<u16>();
        let source = vec![0_u8; byte_count];
        let chunks = [crate::Chunk {
            offset: 0,
            byte_count,
        }];
        let request = crate::DecodeRequest {
            source: &source,
            width: len,
            height: 1,
            chunk_width: len,
            chunk_height: 1,
            chunks: &chunks,
            bits_per_sample: 16,
            sample_format: crate::SampleFormat::Integer,
            byte_order: crate::ByteOrder::Little,
            predictor: crate::Predictor::None,
        };
        reset_tracking(byte_count, align_of::<u16>());

        let image = crate::uncompressed::decode(&request).unwrap();
        let samples = image.into_raw_u16().unwrap();
        assert_eq!(samples.len(), len);

        #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
        {
            assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 1);
        };
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(samples);
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[cfg(feature = "deflate")]
    #[test]
    fn fused_deflate_float_output_uses_the_shared_allocation_policy() {
        use std::io::Write;

        let _guard = TEST_LOCK.lock().unwrap();
        let len = LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<f32>() + 1;
        let byte_count = len * size_of::<f32>();
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&vec![0_u8; byte_count]).unwrap();
        let source = encoder.finish().unwrap();
        let chunks = [crate::Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = crate::DecodeRequest {
            source: &source,
            width: len,
            height: 1,
            chunk_width: len,
            chunk_height: 1,
            chunks: &chunks,
            bits_per_sample: 32,
            sample_format: crate::SampleFormat::Float,
            byte_order: crate::ByteOrder::Little,
            predictor: crate::Predictor::None,
        };
        reset_tracking(byte_count, align_of::<f32>());

        let image = crate::deflate::decode(&request).unwrap();
        let samples = image.into_raw_f32().unwrap();
        assert_eq!(samples.len(), len);

        #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
        {
            assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 1);
        };
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(samples);
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[cfg(feature = "lossless-jpeg")]
    #[test]
    fn lossless_jpeg_full_image_output_uses_the_shared_allocation_policy() {
        let _guard = TEST_LOCK.lock().unwrap();
        let width = 4096;
        let height = LARGE_ALLOCATION_THRESHOLD_BYTES / size_of::<u16>() / width + 1;
        let len = width * height;
        let byte_count = len * size_of::<u16>();
        let samples = vec![1_u16 << 11; len];
        let source =
            crate::ljpeg::encode_interleaved_for_test(&samples, width as u16, height as u16, 1, 12);
        let chunks = [crate::Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = crate::DecodeRequest {
            source: &source,
            width,
            height,
            chunk_width: width,
            chunk_height: height,
            chunks: &chunks,
            bits_per_sample: 12,
            sample_format: crate::SampleFormat::Integer,
            byte_order: crate::ByteOrder::Little,
            predictor: crate::Predictor::None,
        };
        reset_tracking(byte_count, align_of::<u16>());

        let image = crate::ljpeg::decode(&request).unwrap();
        let samples = image.into_raw_u16().unwrap();
        assert_eq!(samples.len(), len);

        #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
        {
            assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 1);
        };
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(samples);
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[cfg(feature = "lossless-jpeg")]
    #[test]
    fn lossless_jpeg_interleaved_output_uses_the_shared_allocation_policy() {
        let _guard = TEST_LOCK.lock().unwrap();
        let width = 2048;
        let height = LARGE_ALLOCATION_THRESHOLD_BYTES / (2 * size_of::<u16>()) / width + 1;
        let input_samples = width * height * 2;
        let samples = vec![1_u16 << 11; input_samples];
        let source =
            crate::ljpeg::encode_interleaved_for_test(&samples, width as u16, height as u16, 2, 12);
        let output_samples = 2 * width * height;
        let byte_count = output_samples * size_of::<u16>();
        reset_tracking(byte_count, align_of::<u16>());

        let image = crate::ljpeg::decode_interleaved(&source, 12).unwrap();
        assert_eq!(image.width, 2 * width);
        assert_eq!(image.height, height);
        assert_eq!(image.samples.len(), output_samples);

        #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
        {
            assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 1);
        };
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(image);
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[cfg(feature = "lossless-jpeg")]
    #[test]
    fn lossless_jpeg_u8_conversion_uses_the_shared_allocation_policy() {
        let _guard = TEST_LOCK.lock().unwrap();
        let width = 4096;
        let height = LARGE_ALLOCATION_THRESHOLD_BYTES / width + 1;
        let len = width * height;
        let samples = vec![1_u16 << 7; len];
        let source =
            crate::ljpeg::encode_interleaved_for_test(&samples, width as u16, height as u16, 1, 8);
        let chunks = [crate::Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = crate::DecodeRequest {
            source: &source,
            width,
            height,
            chunk_width: width,
            chunk_height: height,
            chunks: &chunks,
            bits_per_sample: 8,
            sample_format: crate::SampleFormat::Integer,
            byte_order: crate::ByteOrder::Little,
            predictor: crate::Predictor::None,
        };
        reset_tracking(len, align_of::<u8>());

        let image = crate::ljpeg::decode(&request).unwrap();
        let samples = image.into_raw_u8().unwrap();
        assert_eq!(samples.len(), len);
        assert_eq!(samples.capacity(), len);

        #[cfg(all(target_os = "linux", not(miri), feature = "large-page-hint"))]
        {
            assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(BYPASS_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 0);
            assert_eq!(POLICY_ZEROED_ALLOCATIONS.load(Ordering::SeqCst), 1);
        };
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 0);

        drop(samples);
        assert_eq!(DEALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(DEALLOCATION_LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
        TRACKED_SIZE.store(0, Ordering::SeqCst);
    }

    #[test]
    fn final_decode_outputs_do_not_reintroduce_direct_sample_vecs() {
        let guarded_sources = [
            (
                "samples.rs",
                include_str!("samples.rs"),
                &[
                    "let mut output = vec![0_u8; sample_count]",
                    "let mut output = vec![0_u16; sample_count]",
                    "let mut output = vec![0_u32; sample_count]",
                    "let mut output = vec![0_f32; sample_count]",
                    "let mut output = vec![0_f64; sample_count]",
                ][..],
            ),
            (
                "deflate.rs",
                include_str!("deflate.rs"),
                &["let mut output = vec![0.0_f32; sample_count]"][..],
            ),
            (
                "ljpeg.rs",
                include_str!("ljpeg.rs"),
                &["let mut output = vec![0_u16; sample_count]"][..],
            ),
            (
                "jpeg.rs",
                include_str!("jpeg.rs"),
                &["let mut output = vec![0_u8; sample_count]"][..],
            ),
            (
                "jpegxl.rs",
                include_str!("jpegxl.rs"),
                &[
                    "let mut output = vec![0_u8; sample_count]",
                    "let mut output = vec![0_u16; sample_count]",
                ][..],
            ),
        ];

        for (file, source, forbidden_patterns) in guarded_sources {
            for pattern in forbidden_patterns {
                assert!(
                    !source.contains(pattern),
                    "{file} contains a final-output allocation outside large_alloc: {pattern}"
                );
            }
        }
    }
}
