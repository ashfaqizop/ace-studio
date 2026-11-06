//! Wire primitives matching Java's `DataOutput`/`DataInput` (Guava `ByteArrayDataOutput`), which is what Plasmo
//! Voice writes on both its plugin channel and its UDP socket: big-endian numbers and *modified* UTF-8 strings
//! with a u16 byte-length prefix. (Plasmo Voice: `su.plo.voice.proto.packets.PacketUtil`.)

/// A Java UUID: most and least significant bits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Id(pub u64, pub u64);

impl Id {
    pub fn from_api(u: pumpkin_plugin_api::uuid::Uuid) -> Self {
        Id(u.high, u.low)
    }

    pub fn to_api(self) -> pumpkin_plugin_api::uuid::Uuid {
        pumpkin_plugin_api::uuid::Uuid { high: self.0, low: self.1 }
    }

    pub fn random() -> Self {
        Self::from_api(pumpkin_plugin_api::uuid::generate())
    }
}

#[derive(Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn new() -> Self {
        Out(Vec::with_capacity(64))
    }
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(v as u8)
    }
    pub fn i16(&mut self, v: i16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i64(&mut self, v: i64) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.0.extend_from_slice(&v.to_bits().to_be_bytes());
        self
    }
    pub fn id(&mut self, v: Id) -> &mut Self {
        self.0.extend_from_slice(&v.0.to_be_bytes());
        self.0.extend_from_slice(&v.1.to_be_bytes());
        self
    }
    /// `DataOutput.writeUTF`: modified UTF-8 (NUL as C0 80, supplementary characters as two 3-byte surrogates).
    pub fn utf(&mut self, s: &str) -> &mut Self {
        let mut enc = Vec::with_capacity(s.len());
        for unit in s.encode_utf16() {
            let c = unit as u32;
            if (1..=0x7f).contains(&c) {
                enc.push(c as u8);
            } else if c <= 0x7ff {
                enc.push((0xc0 | (c >> 6)) as u8);
                enc.push((0x80 | (c & 0x3f)) as u8);
            } else {
                enc.push((0xe0 | (c >> 12)) as u8);
                enc.push((0x80 | ((c >> 6) & 0x3f)) as u8);
                enc.push((0x80 | (c & 0x3f)) as u8);
            }
        }
        // Java throws past 65535 bytes; never send anything that long, cut it instead
        enc.truncate(u16::MAX as usize);
        self.u16(enc.len() as u16);
        self.0.extend_from_slice(&enc);
        self
    }
    /// `PacketUtil.writeBytes`: i32 length, then the bytes.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.i32(b.len() as i32);
        self.0.extend_from_slice(b);
        self
    }
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

#[derive(Debug)]
pub struct Bad;

pub type R<T> = Result<T, Bad>;

pub struct In<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> In<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        In { b, p: 0 }
    }
    pub fn take(&mut self, n: usize) -> R<&'a [u8]> {
        let end = self.p.checked_add(n).ok_or(Bad)?;
        let s = self.b.get(self.p..end).ok_or(Bad)?;
        self.p = end;
        Ok(s)
    }
    #[cfg(test)]
    pub fn remaining(&self) -> usize {
        self.b.len() - self.p
    }
    pub fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn bool(&mut self) -> R<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn i16(&mut self) -> R<i16> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().map_err(|_| Bad)?))
    }
    pub fn u16(&mut self) -> R<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().map_err(|_| Bad)?))
    }
    pub fn i32(&mut self) -> R<i32> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().map_err(|_| Bad)?))
    }
    pub fn i64(&mut self) -> R<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().map_err(|_| Bad)?))
    }
    pub fn id(&mut self) -> R<Id> {
        let hi = u64::from_be_bytes(self.take(8)?.try_into().map_err(|_| Bad)?);
        let lo = u64::from_be_bytes(self.take(8)?.try_into().map_err(|_| Bad)?);
        Ok(Id(hi, lo))
    }
    /// `PacketUtil.readSafeInt`.
    pub fn int_in(&mut self, min: i32, max: i32) -> R<i32> {
        let v = self.i32()?;
        if v < min || v > max { Err(Bad) } else { Ok(v) }
    }
    /// `DataInput.readUTF` (modified UTF-8), rejecting strings longer than `max` UTF-16 units.
    pub fn utf(&mut self, max: usize) -> R<String> {
        let len = self.u16()? as usize;
        let raw = self.take(len)?;
        let mut units: Vec<u16> = Vec::with_capacity(len);
        let mut i = 0;
        while i < raw.len() {
            let a = raw[i] as u32;
            let (c, n) = if a < 0x80 {
                (a, 1)
            } else if a & 0xe0 == 0xc0 {
                let b = *raw.get(i + 1).ok_or(Bad)? as u32;
                (((a & 0x1f) << 6) | (b & 0x3f), 2)
            } else if a & 0xf0 == 0xe0 {
                let b = *raw.get(i + 1).ok_or(Bad)? as u32;
                let c = *raw.get(i + 2).ok_or(Bad)? as u32;
                (((a & 0x0f) << 12) | ((b & 0x3f) << 6) | (c & 0x3f), 3)
            } else {
                return Err(Bad);
            };
            units.push(c as u16);
            i += n;
        }
        if units.len() > max {
            return Err(Bad);
        }
        Ok(String::from_utf16_lossy(&units))
    }
    /// `PacketUtil.readBytes`-style: i32 length within `min..=max`, then the bytes.
    pub fn bytes(&mut self, min: i32, max: i32) -> R<Vec<u8>> {
        let n = self.int_in(min, max)? as usize;
        Ok(self.take(n)?.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modified_utf8_round_trip() {
        for s in ["", "plasmo:voice/v2", "Größe", "日本語", "a\0b", "😀 emoji"] {
            let mut o = Out::new();
            o.utf(s);
            let v = o.into_vec();
            assert_eq!(In::new(&v).utf(1000).unwrap(), s);
        }
        // NUL is the two-byte form and emoji are surrogate pairs, like Java
        let mut o = Out::new();
        o.utf("\0😀");
        assert_eq!(o.into_vec(), [0, 8, 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80]);
    }

    #[test]
    fn numbers_are_big_endian() {
        let mut o = Out::new();
        o.i32(0x4e9004e9).i16(-2).id(Id(1, 2));
        let v = o.into_vec();
        let mut i = In::new(&v);
        assert_eq!(i.i32().unwrap(), 0x4e9004e9);
        assert_eq!(i.i16().unwrap(), -2);
        assert_eq!(i.id().unwrap(), Id(1, 2));
        assert_eq!(i.remaining(), 0);
        assert!(i.u8().is_err());
    }
}
