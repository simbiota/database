use crate::database::ObjectCompressionType::DEFLATE;
use crate::database::{Object, ObjectCompressionType, ObjectImpl};
use crate::formats::simple_tlsh::SimpleTLSHEntryType::{Hex, Raw};
use std::fmt::Write;
use std::num::ParseIntError;

pub enum SimpleTLSHEntryType {
    Hex(String),
    Raw(Vec<u8>),
}

impl SimpleTLSHEntryType {
    pub const HEX: Self = Hex(String::new());
    pub const RAW: Self = Raw(Vec::new());

    const fn as_value(&self) -> u16 {
        match self {
            Hex(_) => 0,
            Raw(_) => 1,
        }
    }
    const fn entry_len(&self) -> u16 {
        match self {
            Hex(_) => 70,
            Raw(_) => 35,
        }
    }
}

/// Object format 0x0001, SimpleTLSH.
///
/// Stores a list of the standard 35-byte TLSH hashes either in hexadecimal or in binary format.
pub struct SimpleTLSHObject {
    entries: Vec<SimpleTLSHEntryType>,
    entry_type: SimpleTLSHEntryType,
    compressed: bool,
}

impl SimpleTLSHObject {
    /// Get the stored hashes as hex Strings.
    pub fn get_hashes(&self) -> Vec<String> {
        if matches!(self.entry_type, Hex(_)) {
            return self
                .entries
                .iter()
                .map(|e| {
                    let Hex(s) = e else { panic!("invalid entry") };
                    s.clone()
                })
                .collect();
        }
        todo!();
    }
}

impl ObjectImpl for SimpleTLSHObject {
    const NAME: &'static str = "SimpleTLSH";

    fn to_object(self) -> Object {
        Object {
            format: 0x0001,
            compression_type: ObjectCompressionType::NoCompression,
            entry_type: self.entry_type.as_value(),
            entry_size: 70,
            data: self
                .entries
                .into_iter()
                .map(|e| {
                    let Hex(s) = e else {
                    panic!("invalid entry");
                };
                    s.as_bytes().to_vec()
                })
                .collect(),
        }
    }

    fn from_object(obj: Object) -> Option<Self> {
        if obj.format != 0x0001 {
            return None;
        }
        let format = if obj.entry_type == 0 {
            SimpleTLSHEntryType::HEX
        } else if obj.entry_type == 1 {
            SimpleTLSHEntryType::RAW
        } else {
            return None;
        };
        assert_eq!(obj.entry_size, format.entry_len());
        let mut entries = Vec::new();
        for entry in obj.data {
            if matches!(format, SimpleTLSHEntryType::Hex(_)) {
                entries.push(Hex(String::from_utf8(entry).unwrap()))
            } else {
                entries.push(Raw(entry))
            }
        }
        Some(Self {
            entries,
            entry_type: format,
            compressed: matches!(obj.compression_type, DEFLATE),
        })
    }
}

impl SimpleTLSHObject {
    /// New, empty TLSH list. The object will store the hashes in the
    /// provided format.
    pub fn new(entry_type: SimpleTLSHEntryType) -> Self {
        Self {
            entries: Vec::new(),
            entry_type,
            compressed: false,
        }
    }

    /// New, empty TLSH list with compressed storage enabled.
    pub fn new_compressed(entry_type: SimpleTLSHEntryType) -> Self {
        Self {
            entries: Vec::new(),
            entry_type,
            compressed: true,
        }
    }

    /// Enable or disable object compression for this object.
    pub fn set_compressed(&mut self, compressed: bool) {
        self.compressed = compressed;
    }

    /// Add a hex String hash to the database. If the specified storage mode was RAW, the hash will
    /// be converted to binary.
    pub fn add_hash(&mut self, hash: String) {
        if hash.len() != 70 {
            panic!("invalid tlsh hash");
        }

        if matches!(self.entry_type, SimpleTLSHEntryType::Raw(_)) {
            self.entries.push(Raw(decode_hex(&hash).unwrap()));
        } else {
            self.entries.push(Hex(hash));
        }
    }

    /// Add a raw bytes hash to the database. If the specified storage mode was HEX, the hash will
    /// be converted to hexadecimal format.
    pub fn add_raw_hash(&mut self, hash: &[u8]) {
        if hash.len() != 35 {
            panic!("invalid tlsh hash");
        }
        if matches!(self.entry_type, SimpleTLSHEntryType::Raw(_)) {
            self.entries.push(Raw(hash.to_vec()));
        } else {
            self.entries.push(Hex(encode_hex(hash)));
        }
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}
