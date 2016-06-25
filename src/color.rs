use std::time::Duration;
use keys::*;
use handle::{ToControlPacket, ControlPacket};
use byteorder::{BigEndian, WriteBytesExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyColor {
    pub key: Key,
    pub color: Color,
}

impl KeyColor {
    pub fn new<T: Into<Key>>(key: T, color: Color) -> KeyColor {
        KeyColor {
            key: key.into(),
            color: color,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPacket<T: KeyType> {
    colors: Vec<(T, Color)>,
}

impl<T: KeyType> ColorPacket<T> {
    pub fn new() -> ColorPacket<T> {
        ColorPacket {
            colors: Vec::new(),
        }
    }

    /// Adds a color.
    ///
    /// If a packet is already full and therefore should be sent,
    /// it will be returned.
    /// This instance will be emptied, and given key with it's color will then be added.
    /// That is why new colors can then be added to this struct again.
    ///
    /// Otherwise None will be returned and more colors can be added to this instance.
    pub fn add(&mut self, key: T, color: Color) -> Option<ColorPacket<T>> {
        assert!(self.colors.len() <= 14);
        let res = if self.colors.len() == 14 {
            Some(::std::mem::replace(self, ColorPacket::new()))
        } else {
            None
        };
        self.colors.push((key, color));
        res
    }

    /// Returns the number of Colors in this packet
    pub fn len(&self) -> usize {
        self.colors.len()
    }
}

impl<T: KeyType> ToControlPacket for ColorPacket<T> {
    fn to_control_packet(mut self) -> ControlPacket {
        let mut buf = Vec::new();
        // head
        buf.write_u32::<BigEndian>(0x12ff0f3b).unwrap();
        // key type
        // if none is specified, no data exists and no key will be set
        // as From can not return a Result, just use any key type
        buf.write_u16::<BigEndian>(T::id()).unwrap();
        // reserved
        buf.write_u8(0x00).unwrap();
        // number of key colors
        buf.write_u8(self.colors.len() as u8).unwrap();
        // key colors
        for (key, color) in self.colors.drain(..) {
            buf.write_u8(key.raw_value()).unwrap();
            buf.write_u8(color.red).unwrap();
            buf.write_u8(color.green).unwrap();
            buf.write_u8(color.blue).unwrap();
        }
        // pad rest if needed
        buf.resize(64, 0u8);
        ControlPacket::new(buf, 0x80, 0x21, 9, 0x0212, 0x0001, Duration::from_secs(10))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlushPacket;

impl FlushPacket {
    pub fn new() -> FlushPacket {
        FlushPacket {  }
    }
}

impl ToControlPacket for FlushPacket {
    fn to_control_packet(self) -> ControlPacket {
        let mut buf = Vec::new();
        // head
        buf.write_u32::<BigEndian>(0x11ff0f5b).unwrap();
        // body is 0
        buf.resize(20, 0u8);
        ControlPacket::new(buf, 0x80, 0x21, 9, 0x0212, 0x0001, Duration::from_secs(10))
    }
}

