pub fn mnt_to_string(bytes: &[i8]) -> String {
    let valid_bytes: Vec<u8> = bytes.iter().map(|&x| x as u8).collect();
   let result = String::from_utf8_lossy(&valid_bytes).to_string();
    let result = result.replace('\0', " ");
    result.trim_end().to_string()
}
