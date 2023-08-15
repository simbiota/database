use crate::database::{Object, ObjectCompressionType, ObjectImpl};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoredTLSHWithDistanceEntry {
    pub tlsh_bytes: [u8; 36],
    pub sha256_hash: [u8; 32],
    pub distance: u8,
}

/// Object format 0x0003, ColoredTLSHWithDistance.
///
/// Stores a list of the standard 35-byte TLSH hashes int binary format with a SHA256 hash and a detection distance.
pub struct ColoredTLSHWithDistanceObject {
    entries: Vec<ColoredTLSHWithDistanceEntry>,
}

impl ColoredTLSHWithDistanceObject {
    pub fn empty() -> Self {
        Self { entries: vec![] }
    }

    pub fn get_entries(&self) -> &Vec<ColoredTLSHWithDistanceEntry> {
        &self.entries
    }

    pub fn add_entry(&mut self, tlsh_hash: &[u8], sha_hash: &[u8], distance: u8) {
        self.entries.push(ColoredTLSHWithDistanceEntry {
            tlsh_bytes: tlsh_hash.try_into().unwrap(),
            sha256_hash: sha_hash.try_into().unwrap(),
            distance,
        });
    }
}

impl ObjectImpl for ColoredTLSHWithDistanceObject {
    const NAME: &'static str = "ColoredTLSHWithDistance";

    fn to_object(self) -> Object {
        Object {
            format: 0x0003,
            compression_type: ObjectCompressionType::NoCompression,
            entry_type: 0,
            entry_size: 36 + 32 + 1,
            data: self
                .entries
                .into_iter()
                .map(|e| {
                    let mut e_vec = e.tlsh_bytes.to_vec();
                    e.sha256_hash.into_iter().for_each(|e| e_vec.push(e));
                    e_vec.push(e.distance);
                    e_vec
                })
                .collect(),
        }
    }

    fn from_object(obj: Object) -> Option<Self>
        where
            Self: Sized,
    {
        if obj.format != 0x0003 {
            return None;
        }

        let mut entries = Vec::new();
        for entry in obj.data {
            let e = ColoredTLSHWithDistanceEntry {
                tlsh_bytes: entry[0..36].try_into().unwrap(),
                sha256_hash: entry[36..36 + 32].try_into().unwrap(),
                distance: entry[36 + 32],
            };
            entries.push(e);
        }

        Some(Self { entries })
    }
}