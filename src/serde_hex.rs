// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub(crate) trait SerdeHex {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    where
        Self: Sized;
}

#[derive(Deserialize, Serialize)]
struct SerdeHexWrapper<T: SerdeHex>(#[serde(with = "SerdeHex")] T);

fn serialize_ref_helper<T: SerdeHex, S: Serializer>(
    v: &&T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    v.serialize(serializer)
}

#[derive(Serialize)]
struct SerdeHexRefWrapper<'a, T: SerdeHex>(#[serde(serialize_with = "serialize_ref_helper")] &'a T);

impl<T: SerdeHex> SerdeHex for Option<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_ref().map(SerdeHexRefWrapper).serialize(serializer)
    }
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    where
        Self: Sized,
    {
        Ok(Option::<SerdeHexWrapper<T>>::deserialize(deserializer)?.map(|v| v.0))
    }
}

macro_rules! impl_hex_for_uint {
    ($ty:ty) => {
        impl SerdeHex for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&format!("{:#X}", self))
            }
            fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text: &str = Deserialize::deserialize(deserializer)?;
                const PREFIX: &str = "0x";
                if text.starts_with(PREFIX) {
                    let hex_digits = &text[PREFIX.len()..];
                    Self::from_str_radix(hex_digits, 16).map_err(de::Error::custom)
                } else {
                    Err(de::Error::custom("hexadecimal field must start with 0x"))
                }
            }
        }
    };
}

impl_hex_for_uint!(u8);
impl_hex_for_uint!(u16);
impl_hex_for_uint!(u32);
impl_hex_for_uint!(u64);
impl_hex_for_uint!(u128);
