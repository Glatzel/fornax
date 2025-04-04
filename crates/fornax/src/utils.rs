pub fn mnt_to_string(bytes: &[i8]) -> String {
    let result: String = bytes
        .iter()
        .map(|c| char::from(*c as u8))
        .collect::<String>();
    let result = result.replace('\0', " ");
    result.trim_end().to_string()
}
