// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use serde::{Deserialize, Deserializer, Serializer};

pub(crate) trait SerdeHex {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    where
        Self: Sized;
}

impl SerdeHex for u64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:#X}", self))
    }
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let _text: &str = Deserialize::deserialize(deserializer)?;
        todo!("parse text as 0x<HEXDIGITS>")
    }
}
