pub fn mnt_to_string(bytes: &[i8]) -> String {
    unsafe { std::str::from_utf8_unchecked(std::mem::transmute::<&[i8], &[u8]>(bytes)) }.to_string()
}
