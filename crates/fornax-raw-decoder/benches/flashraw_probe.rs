#[cfg(target_os = "linux")]
#[path = "flashraw_probe/linux.rs"]
mod linux;

fn main() {
    #[cfg(target_os = "linux")]
    linux::main();

    #[cfg(not(target_os = "linux"))]
    eprintln!(
        "flashraw_probe skipped: Linux-only huge-page advice and /proc smaps probes are unavailable"
    );
}
