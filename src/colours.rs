#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextColour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl TextColour {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub fn from_hex(hex: &str) -> Self {
        let mut hex = hex;
        if hex.starts_with('#') {
            hex = &hex[1..];
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        Self { r, g, b, a: 255 }
    }
    pub fn from_hex_with_alpha(hex: &str) -> Self {
        let mut hex = hex;
        if hex.starts_with('#') {
            hex = &hex[1..];
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        let a = u8::from_str_radix(&hex[6..8], 16).unwrap();
        Self { r, g, b, a }
    }
}