pub fn mnt_to_string(bytes: &[i8]) -> String {
   unsafe { std::str::from_utf8_unchecked(std::mem::transmute(bytes)) }.to_string()
}