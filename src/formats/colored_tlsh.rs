use crate::database::{Object, ObjectCompressionType, ObjectImpl};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoredTLSHEntry {
    pub tlsh_bytes: [u8; 36],
    pub sha256_hash: [u8; 32],
}

/// Object format 0x0002, ColoredTLSH.
///
/// Stores a list of the standard 35-byte TLSH hashes int binary format.
pub struct ColoredTLSHObject {
    entries: Vec<ColoredTLSHEntry>,
}

impl ColoredTLSHObject {
    pub fn empty() -> Self {
        Self { entries: vec![] }
    }

    pub fn get_entries(&self) -> &Vec<ColoredTLSHEntry> {
        &self.entries
    }

    pub fn add_entry(&mut self, tlsh_hash: &[u8], sha_hash: &[u8]) {
        self.entries.push(ColoredTLSHEntry {
            tlsh_bytes: tlsh_hash.try_into().unwrap(),
            sha256_hash: sha_hash.try_into().unwrap(),
        });
    }
}

impl ObjectImpl for ColoredTLSHObject {
    const NAME: &'static str = "ColoredTLSH";

    fn to_object(self) -> Object {
        Object {
            format: 0x0002,
            compression_type: ObjectCompressionType::NoCompression,
            entry_type: 0,
            entry_size: 36 + 32,
            data: self
                .entries
                .into_iter()
                .map(|e| {
                    let mut e_vec = e.tlsh_bytes.to_vec();
                    e.sha256_hash.into_iter().for_each(|e| e_vec.push(e));
                    e_vec
                })
                .collect(),
        }
    }

    fn from_object(obj: Object) -> Option<Self>
    where
        Self: Sized,
    {
        if obj.format != 0x0002 {
            return None;
        }

        let mut entries = Vec::new();
        for entry in obj.data {
            let e = ColoredTLSHEntry {
                tlsh_bytes: entry[0..36].try_into().unwrap(),
                sha256_hash: entry[36..36 + 32].try_into().unwrap(),
            };
            entries.push(e);
        }

        Some(Self { entries })
    }
}
