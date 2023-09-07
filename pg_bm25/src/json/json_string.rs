use pgrx::{Json, JsonB};
use serde_json::json;

use crate::json::timestamp::{
    ParadeDate, ParadeTime, ParadeTimeWithTimeZone, ParadeTimestamp, ParadeTimestampWithTimeZone,
};

pub trait JsonString: Send + Sync {
    fn push_json(&self, target: &mut Vec<u8>);
}

impl JsonString for i16 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for i32 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for i64 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for f32 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for f64 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for u32 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for u64 {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for bool {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.to_string().as_bytes());
    }
}

impl JsonString for ParadeTime {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, self).ok();
    }
}

impl JsonString for ParadeTimeWithTimeZone {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, self).ok();
    }
}

impl JsonString for ParadeTimestamp {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, self).ok();
    }
}

impl JsonString for ParadeTimestampWithTimeZone {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, self).ok();
    }
}

impl JsonString for ParadeDate {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, self).ok();
    }
}

impl JsonString for () {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(b"null");
    }
}

impl JsonString for &str {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, &json!(self)).ok();
    }
}

impl JsonString for String {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, &json!(self)).ok();
    }
}

impl JsonString for pgrx::JsonString {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        if self.0.contains('\r') || self.0.contains('\n') {
            // replace \r\n's to ensure it's all on one line.  It's otherwise supposed to be valid JSON
            // so we shouldn't be mistakenly replacing any \r\n's in actual values -- those should already
            // be properly escaped
            target.extend_from_slice(self.0.replace(['\r', '\n'], " ").as_bytes());
        } else {
            target.extend_from_slice(self.0.as_bytes())
        }
    }
}

impl<T> JsonString for Vec<Option<T>>
where
    T: JsonString,
{
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        target.push(b'[');
        for (i, datum) in self.iter().enumerate() {
            if i > 0 {
                target.push(b',');
            }
            match datum {
                Some(datum) => datum.push_json(target),
                None => target.extend_from_slice(b"null"),
            }
        }
        target.push(b']');
    }
}

impl JsonString for Json {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, &self.0).ok();
    }
}

impl JsonString for JsonB {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, &self.0).ok();
    }
}

impl JsonString for serde_json::Value {
    #[inline]
    fn push_json(&self, target: &mut Vec<u8>) {
        serde_json::to_writer(target, self).ok();
    }
}

impl std::fmt::Debug for Box<dyn JsonString> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let mut s = Vec::new();
        self.push_json(&mut s);
        f.write_str(&unsafe { String::from_utf8_unchecked(s) })
    }
}
