use std::sync::OnceLock;

/// Best-effort L2 size in bytes. `None` if nothing could be detected.
pub fn detect_l2() -> Option<usize> {
    // x86/x86_64: CPUID (Intel leaf 4, AMD leaf 0x8000001D)
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use raw_cpuid::{CacheType, CpuId};
        let size = CpuId::new().get_cache_parameters().and_then(|mut it| {
            it.find(|c| c.level() == 2 && c.cache_type() != CacheType::Instruction)
                .map(|c| {
                    c.associativity()
                        * c.physical_line_partitions()
                        * c.coherency_line_size()
                        * c.sets()
                })
        });
        if size.is_some() {
            return size;
        }
    }

    // Linux (any arch): sysfs. Covers ARM, and x86 if CPUID gave nothing (some
    // VMs).
    #[cfg(target_os = "linux")]
    {
        for i in 0..8 {
            let dir = format!("/sys/devices/system/cpu/cpu0/cache/index{i}");
            let read = |f: &str| std::fs::read_to_string(format!("{dir}/{f}")).ok();
            let Some(level) = read("level") else { break };
            if level.trim() != "2" || read("type").is_some_and(|t| t.trim() == "Instruction") {
                continue;
            }
            let s = read("size")?;
            let s = s.trim(); // "512K", "4M"
            let (num, mul) = match s.chars().last()? {
                'K' => (&s[..s.len() - 1], 1024),
                'M' => (&s[..s.len() - 1], 1024 * 1024),
                _ => (s, 1),
            };
            return num.parse::<usize>().ok().map(|n| n * mul);
        }
    }

    // macOS: Apple Silicon (take the smaller of P/E clusters), Intel Macs fall
    // back to hw.l2cachesize
    #[cfg(target_os = "macos")]
    {
        let get = |name: &str| -> Option<usize> {
            let name = std::ffi::CString::new(name).ok()?;
            let mut val: u64 = 0;
            let mut len = std::mem::size_of::<u64>();
            let rc = unsafe {
                libc::sysctlbyname(
                    name.as_ptr(),
                    &mut val as *mut _ as *mut _,
                    &mut len,
                    std::ptr::null_mut(),
                    0,
                )
            };
            (rc == 0 && val > 0).then_some(val as usize)
        };
        return [
            get("hw.perflevel0.l2cachesize"),
            get("hw.perflevel1.l2cachesize"),
        ]
        .into_iter()
        .flatten()
        .min()
        .or_else(|| get("hw.l2cachesize"));
    }

    unreachable!()
}

/// Cached, clamped, with fallback.
pub fn l2_cache_bytes() -> usize {
    static L2: OnceLock<usize> = OnceLock::new();
    *L2.get_or_init(|| {
        detect_l2()
            .unwrap_or(256 * 1024)
            .clamp(64 * 1024, 2 * 1024 * 1024)
    })
}
