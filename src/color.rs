use keys::*;
use byteorder::{BigEndian, WriteBytesExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyColor {
    key: Key,
    color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPacket {
    key_type: Option<KeyType>,
    colors: Vec<KeyColor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlushPacket {
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyColorError {
    PacketFull,
    InvalidKeyType,
}

impl Color {
    pub fn new(red: u8, green: u8, blue: u8) -> Color {
        Color {
            red: red,
            green: green,
            blue: blue,
        }
    }
}

impl KeyColor {
    pub fn new<T: Into<Key>>(key: T, color: Color) -> KeyColor {
        KeyColor {
            key: key.into(),
            color: color,
        }
    }
}

impl ColorPacket {
    pub fn new() -> ColorPacket {
        ColorPacket {
            key_type: None,
            colors: Vec::new(),
        }
    }

    /// Adds a color to this packet
    ///
    /// If this packet is alredy full, Err will be returned.
    pub fn add_key_color(&mut self, key_color: KeyColor) -> Result<(), KeyColorError> {
        match self.key_type {
            None => self.key_type = Some((&key_color.key).into()),
            Some(key_code) if key_code != (&key_color.key).into() =>
                return Err(KeyColorError::InvalidKeyType),
            _ => {}
        }

        if self.colors.len() >= 14 {
            Err(KeyColorError::PacketFull)
        } else {
            self.colors.push(key_color);
            Ok(())
        }
    }

    /// Wrapper for ColorPacket::into::<[u8; 64]>()
    pub fn to_arr(self) -> [u8; 64] {
        self.into()
    }
}

impl From<ColorPacket> for [u8; 64] {
    fn from(color_packet: ColorPacket) -> [u8; 64] {
        let mut arr = [0u8; 64];
        // head
        (&mut arr[0..4]).write_u32::<BigEndian>(0x12ff0f3b).unwrap();
        // key type
        // if none is specified, no data exists and no key will be set
        // as From can not return a Result, just use any key type
        (&mut arr[4..6]).write_u16::<BigEndian>(color_packet.key_type.unwrap_or(KeyType::Standard) as u16).unwrap();
        // reserved
        arr[6] = 0x00;
        // number of key colors
        arr[7] = color_packet.colors.len() as u8;
        // key colors
        for (key_col, buf) in color_packet.colors.iter().zip((&mut arr[8..64]).chunks_mut(4)) {
            buf[0] = key_col.key.clone().into();
            buf[1] = key_col.color.red;
            buf[2] = key_col.color.green;
            buf[3] = key_col.color.blue;
        }
        arr
    }
}

impl FlushPacket {
    pub fn new() -> FlushPacket {
        FlushPacket {  }
    }

    pub fn to_arr(self) -> [u8; 20] {
        self.into()
    }
}

impl From<FlushPacket> for [u8; 20] {
    #[allow(unused_variables)]
    fn from(p: FlushPacket) -> [u8; 20] {
        let mut arr = [0u8; 20];
        // head
        (&mut arr[0..4]).write_u32::<BigEndian>(0x11ff0f5b).unwrap();
        // body is 0
        arr
    }
}

