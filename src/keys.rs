// http://blog.jwilm.io/racerd/clap/macro.arg_enum!.html
// modified to support value assignment and repr
// values() function added returning a vec of all variants
macro_rules! arg_enum {
    (#[repr($($r:ident),+)] #[derive($($d:ident),+)] pub enum $e:ident { $($v:ident = $val:expr),+ } ) => {
        #[repr($($r,)+)]
        #[derive($($d,)+)]
        pub enum $e {
            $($v = $val),+
        }

        impl ::std::str::FromStr for $e {
            type Err = String;

            fn from_str(s: &str) -> Result<Self,Self::Err> {
                use ::std::ascii::AsciiExt;
                match s {
                    $(stringify!($v) |
                    _ if s.eq_ignore_ascii_case(stringify!($v)) => Ok($e::$v),)+
                    _                => Err({
                                            let v = vec![
                                                $(stringify!($v),)+
                                            ];
                                            format!("valid values:{}",
                                                v.iter().fold(String::new(), |a, i| {
                                                    a + &format!(" {}", i)[..]
                                                }))
                                        })
                }
            }
        }

        impl ::std::convert::From<u8> for $e {
            fn from(u: u8) -> $e {
                match u {
                    $($val => $e::$v,)+
                    _ => $e::None

                }
            }
        }

        impl $e {
            #[allow(dead_code)]
            pub fn values() -> Vec<$e> {
                vec![
                    $($e::$v),+
                ]
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyEvent {
    KeyPressed(Key),
    KeyReleased(Key),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Standard(StandardKey),
    Gaming(GamingKey),
    Logo(Logo),
}

impl<'a> From<&'a Key> for KeyType {
    fn from(key: &Key) -> KeyType {
        match key {
            &Key::Standard(_) => KeyType::Standard,
            &Key::Gaming(_) => KeyType::Gaming,
            &Key::Logo(_) => KeyType::Logo,
        }
    }
}

impl Into<u8> for Key {
    fn into(self) -> u8 {
        match self {
            Key::Standard(s) => s as u8,
            Key::Gaming(g) => g as u8,
            Key::Logo(m) => m as u8,
        }
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyType {
    Standard = 0x0001,
    Gaming = 0x0004,
    Logo = 0x0010,
}

arg_enum! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum StandardKey {
        None = 0x00,
        A = 0x04,
        B = 0x05,
        C = 0x06,
        D = 0x07,
        E = 0x08,
        F = 0x09,
        G = 0x0a,
        H = 0x0b,
        I = 0x0c,
        J = 0x0d,
        K = 0x0e,
        L = 0x0f,
        M = 0x10,
        N = 0x11,
        O = 0x12,
        P = 0x13,
        Q = 0x14,
        R = 0x15,
        S = 0x16,
        T = 0x17,
        U = 0x18,
        V = 0x19,
        W = 0x1a,
        X = 0x1b,
        Z = 0x1c,
        Y = 0x1d,
        _1 = 0x1e,
        _2 = 0x1f,
        _3 = 0x20,
        _4 = 0x21,
        _5 = 0x22,
        _6 = 0x23,
        _7 = 0x24,
        _8 = 0x25,
        _9 = 0x26,
        _0 = 0x27,
        Return = 0x28,
        Esc = 0x29,
        Backspace = 0x2a,
        Tab = 0x2b,
        Space = 0x2c,
        Sz = 0x2d,
        Tick = 0x2e,
        Uuml = 0x2f,
        Plus = 0x30,
        Pipe = 0x31,
        Sharp = 0x32,
        Ouml = 0x33,
        Auml = 0x34,
        Circumflex = 0x35,
        Comma = 0x36,
        Dot = 0x37,
        Minus = 0x38,
        CapsLock = 0x39,
        F1 = 0x3a,
        F2 = 0x3b,
        F3 = 0x3c,
        F4 = 0x3d,
        F5 = 0x3e,
        F6 = 0x3f,
        F7 = 0x40,
        F8 = 0x41,
        F9 = 0x42,
        F10 = 0x43,
        F11 = 0x44,
        F12 = 0x45,
        Print = 0x46,
        ScrollLock = 0x47,
        Pause = 0x48,
        Insert = 0x49,
        Home = 0x4a,
        PageUp = 0x4b,
        Delete = 0x4c,
        End = 0x4d,
        PageDown = 0x4e,
        Right = 0x4f,
        Left = 0x50,
        Down = 0x51,
        Up = 0x52,
        NumLock = 0x53,
        NumSlash = 0x54,
        NumStar = 0x55,
        NumMinus = 0x56,
        NumPlus = 0x57,
        NumReturn = 0x58,
        Num1 = 0x59,
        Num2 = 0x5a,
        Num3 = 0x5b,
        Num4 = 0x5c,
        Num5 = 0x5d,
        Num6 = 0x5e,
        Num7 = 0x5f,
        Num8 = 0x60,
        Num9 = 0x61,
        Num0 = 0x62,
        NumComma = 0x63,
        SmallerThan = 0x64,
        Menu = 0x65,
        International1 = 0x87,
        // gets mapped to 0x87 in firmware
        International2 = 0x88,
        International3 = 0x89,
        International4 = 0x8a,
        International5 = 0x8b,
        LeftControl = 0xe0,
        LeftShift = 0xe1,
        LeftAlt = 0xe2,
        LeftWindows = 0xe3,
        RightControl = 0xe4,
        RightShift = 0xe5,
        RightAlt = 0xe6,
        RightWindows = 0xe7
    }
}

impl From<StandardKey> for Key {
    fn from(standard: StandardKey) -> Key {
        Key::Standard(standard)
    }
}

arg_enum! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum GamingKey {
        None = 0x00,
        G1 = 0x01,
        G2 = 0x02,
        G3 = 0x03,
        G4 = 0x04,
        G5 = 0x05,
        G6 = 0x06,
        G7 = 0x07,
        G8 = 0x08,
        G9 = 0x09
    }
}

impl From<GamingKey> for Key {
    fn from(gaming: GamingKey) -> Key {
        Key::Gaming(gaming)
    }
}

arg_enum! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Logo {
        None = 0x00,
        G = 0x01,
        G910 = 0x02
    }
}

impl From<Logo> for Key {
    fn from(logo: Logo) -> Key {
        Key::Logo(logo)
    }
}
