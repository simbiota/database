use crate::next_multiple_of;
use crate::object::ObjectDecodeError::{CompressionError, TooShort, UnsupportedCompression};

#[cfg(feature = "compression")]
use flate2::read::ZlibDecoder;
#[cfg(feature = "compression")]
use std::io::Read;

#[derive(Debug)]
pub enum ObjectDecodeError {
    TooShort,
    InvalidPadding,
    UnsupportedCompression(u16),
    CompressionError(std::io::Error),
}

#[derive(Debug)]
pub struct RawObject {
    pub format: u16,
    pub compression: u16,
    pub entry_type: u16,
    pub entry_size: u16,
    pub length: u64,
    pub(crate) data: Vec<Vec<u8>>,
}

impl RawObject {
    pub(crate) fn new(format: u16, compression: u16, entry_type: u16, entry_size: u16) -> Self {
        Self {
            format,
            compression,
            entry_type,
            entry_size,
            length: 0,
            data: Vec::new(),
        }
    }

    fn decode_data(compression: u16, input_data: &[u8]) -> Result<Vec<u8>, ObjectDecodeError> {
        match compression {
            0x0000 => Ok(input_data.to_vec()),
            0x0001 => {
                // flate2 deflate
                if cfg!(feature = "compression") {
                    Self::decode_flate2(input_data)
                } else {
                    Err(UnsupportedCompression(0x0001))
                }
            }
            c => Err(UnsupportedCompression(c)),
        }
    }

    #[cfg(feature = "compression")]
    fn decode_flate2(input_data: &[u8]) -> Result<Vec<u8>, ObjectDecodeError> {
        let mut decoder = ZlibDecoder::new(input_data);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .map_err(CompressionError)?;
        Ok(decoded)
    }
}

impl TryFrom<Vec<u8>> for RawObject {
    type Error = ObjectDecodeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<&[u8]> for RawObject {
    type Error = ObjectDecodeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let data_length = value.len();
        if data_length < 2 + 2 + 2 + 2 + 8 {
            return Err(TooShort);
        }

        let format = u16::from_be_bytes((&value[0..2]).try_into().unwrap());
        let compression = u16::from_be_bytes((&value[2..4]).try_into().unwrap());
        let entry_type = u16::from_be_bytes((&value[4..6]).try_into().unwrap());
        let entry_size = u16::from_be_bytes((&value[6..8]).try_into().unwrap());
        let length = u64::from_be_bytes((&value[8..16]).try_into().unwrap());

        if data_length < length as usize {
            return Err(TooShort);
        }
        if length <= 16 {
            return Err(TooShort);
        }
        let data_length = length - (2 + 2 + 2 + 2 + 8);
        let decoded_data = Self::decode_data(compression, &value[16..(16 + data_length) as usize])?;
        let data: Vec<Vec<u8>> = decoded_data
            .chunks_exact(entry_size as usize)
            .map(|c| c.to_vec())
            .collect();

        Ok(Self {
            format,
            compression,
            entry_size,
            entry_type,
            length,
            data,
        })
    }
}

impl From<RawObject> for Vec<u8> {
    fn from(value: RawObject) -> Self {
        let mut data = Vec::with_capacity(16);

        value
            .format
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));

        value
            .compression
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));

        value
            .entry_type
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));

        value
            .entry_size
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));
        let entry_count = value.data.len();
        let raw_length = 16 + (entry_count * value.entry_size as usize);
        let full_length = next_multiple_of(raw_length, 16);
        (raw_length as u64)
            .to_be_bytes()
            .iter()
            .for_each(|b| data.push(*b));
        let padding_len = full_length - raw_length;
        // TODO: Compress
        assert_eq!(value.compression, 0);
        for entry in &value.data {
            assert_eq!(entry.len(), value.entry_size as usize);
            entry.iter().for_each(|b| data.push(*b));
        }

        // Add padding
        (0..padding_len).for_each(|_| data.push(0));

        data
    }
}

#[cfg(test)]
mod test {
    use crate::object::{ObjectDecodeError, RawObject};

    #[test]
    pub fn test_object_load() {
        let data_raw = b"\x00\x01\x00\x00\x00\x01\x00\x10\x00\x00\x00\x00\x00\x00\x00\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02";
        let object = RawObject::try_from(data_raw as &[u8]).unwrap();
        assert_eq!(object.format, 0x01);
        assert_eq!(object.compression, 0x00);
        assert_eq!(object.entry_type, 0x01);
        assert_eq!(object.entry_size, 16);
        assert_eq!(object.length, 0x30);
        assert_eq!(object.data.len(), 2);

        let too_short = b"\x00\x01\x00\x00\x00\x01\x00\x10\x00\x00\x00\x00\x00\x00\x00\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let object = RawObject::try_from(too_short as &[u8]).unwrap_err();
        assert!(matches!(object, ObjectDecodeError::TooShort));

        let not_padded = b"\x00\x01\x00\x00\x00\x01\x00\x06\x00\x00\x00\x00\x00\x00\x00\x1c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let object = RawObject::try_from(not_padded as &[u8]).unwrap_err();
        assert!(matches!(object, ObjectDecodeError::InvalidPadding));
    }

    #[test]
    pub fn test_object_save() {
        let data_raw = b"\x00\x01\x00\x00\x00\x01\x00\x10\x00\x00\x00\x00\x00\x00\x00\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02";
        let mut object1 = RawObject::new(0x01, 0x00, 0x01, 0x10);
        object1.add_data(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        object1.add_data(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(Vec::from(object1).as_slice(), data_raw);

        let data_raw_padded = b"\x00\x01\x00\x00\x00\x01\x00\x06\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x02\x00\x00\x00\x00";
        let mut object2 = RawObject::new(0x01, 0x00, 0x01, 0x6);
        object2.add_data(vec![0, 0, 0, 0, 0, 1]);
        object2.add_data(vec![0, 0, 0, 0, 0, 2]);

        assert_eq!(Vec::from(object2).as_slice(), data_raw_padded);
    }
}
