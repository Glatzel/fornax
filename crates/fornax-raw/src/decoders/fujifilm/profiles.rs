#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelProfile {
    pub cfa: [u8; 4],
    pub borders: [u32; 4],
}

pub fn profile(model: &str) -> Option<ModelProfile> {
    let key: String = model
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_uppercase)
        .collect();
    let borders = match key.as_str() {
        "GFX50S" | "GFX50R" => [0, 0, 936, 2],
        "GFX50SII" => [0, 0, 192, 2],
        "GFX100" | "GFX100S" => [0, 4, 144, 2],
        "GFX100II" => [0, 2, 144, 4],
        "GFX100SII" | "GFX100RF" => [0, 0, 144, 4],
        _ => return None,
    };
    Some(ModelProfile {
        cfa: [0, 1, 1, 2],
        borders,
    })
}
