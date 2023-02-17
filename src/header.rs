use crate::header::HeaderDecodeError::{
    InvalidMagic, InvalidPadding, TooShort, UnsupportedVersion,
};
use crate::next_multiple_of;

pub const HEADER_MAGIC: [u8; 4] = [0x43, 0x53, 0x47, 0x4d]; // ASCII 'CSGM'

#[derive(Debug)]
pub enum HeaderDecodeError {
    InvalidMagic,
    TooShort,
    UnsupportedVersion,
    InvalidPadding,
}

#[derive(Debug)]
pub struct Header {
    pub version: u32,
    pub number_of_objects: u64,
    pub header_len: u32,
    pub extra_data: Vec<u8>,
}

impl Header {
    /// Reads the version from the database bytes
    ///
    /// This can be used for determining the header layout based on the version of
    /// the database file
    ///
    /// Note: This function checks the magic value and bails out if it is not the expected value
    pub(crate) fn partial_version(data: &[u8]) -> Result<u32, HeaderDecodeError> {
        if data.len() < 4 + 4 {
            // magic + version
            return Err(TooShort);
        }
        let magic_bytes = &data[0..4];
        if magic_bytes != HEADER_MAGIC {
            return Err(InvalidMagic);
        }
        let version_bytes = u32::from_be_bytes((&data[4..8]).try_into().unwrap());
        Ok(version_bytes)
    }

    pub(crate) fn new(number_of_objects: u64, extra_data: Vec<u8>) -> Self {
        Self {
            version: 1,
            number_of_objects,
            header_len: 0,
            extra_data,
        }
    }
}

impl From<Header> for Vec<u8> {
    fn from(value: Header) -> Self {
        let mut data: Vec<u8> = Vec::new();

        // Push magic
        HEADER_MAGIC.iter().for_each(|v| data.push(*v));
        value
            .version
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));
        value
            .number_of_objects
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));

        let length_before_padding = 4 + 4 + 8 + 4 + value.extra_data.len();
        let full_length = next_multiple_of(length_before_padding, 16);
        let padding_length = full_length - length_before_padding;

        (full_length as u32)
            .to_be_bytes()
            .iter()
            .for_each(|v| data.push(*v));
        value.extra_data.iter().for_each(|v| data.push(*v));

        // Add padding
        (0..padding_length).for_each(|_| data.push(0));
        assert_eq!(data.len(), full_length);

        data
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = HeaderDecodeError;

    /// Try to parse header from a byte array
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let input_length = value.len();
        if input_length < 4 + 4 + 8 + 4 {
            return Err(TooShort);
        }

        let version = Header::partial_version(value)?;
        if version != 1 {
            return Err(UnsupportedVersion);
        }

        let number_of_objects = u64::from_be_bytes((&value[8..16]).try_into().unwrap());
        let header_length = u32::from_be_bytes((&value[16..20]).try_into().unwrap());

        if input_length < header_length as usize {
            return Err(TooShort);
        }

        if header_length % 16 != 0 {
            return Err(InvalidPadding);
        }

        let mut extra_data = Vec::new();
        let remaining_bytes = header_length - (4 + 4 + 8 + 4);

        for offset in 0..remaining_bytes {
            extra_data.push(value[20 + offset as usize]);
        }

        Ok(Self {
            version,
            number_of_objects,
            header_len: header_length,
            extra_data,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::header::{Header, HeaderDecodeError};

    #[test]
    pub fn test_partial_version() {
        let valid_version_1 = b"CSGM\x00\x00\x00\x01";
        let valid_version_2 = b"CSGM\x00\x00\x00\x02";
        let valid_version_big = b"CSGM\x01\x00\x00\x00";
        let invalid_magic = b"CSBM\x00\x00\x00\x01";
        let invalid_too_short = b"CSGM\x01\x00\x00";

        let version_1 = Header::partial_version(valid_version_1).unwrap();
        assert_eq!(version_1, 1);
        let version_2 = Header::partial_version(valid_version_2).unwrap();
        assert_eq!(version_2, 2);
        let version_big = Header::partial_version(valid_version_big).unwrap();
        assert_ne!(version_big, 1);
        assert_eq!(version_big, 16777216);

        let version_invalid_magic = Header::partial_version(invalid_magic).unwrap_err();
        assert!(matches!(
            version_invalid_magic,
            HeaderDecodeError::InvalidMagic
        ));
        let version_too_short = Header::partial_version(invalid_too_short).unwrap_err();
        assert!(matches!(version_too_short, HeaderDecodeError::TooShort));
    }

    #[test]
    pub fn test_header_from_bytes() {
        let valid_header = b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let invalid_version = b"CSGM\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let invalid_magic = b"CSBM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let too_short = b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let invalid_padding =
            b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x14";

        let header = Header::try_from(valid_header as &[u8]).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.header_len, 32);
        assert_eq!(header.number_of_objects, 1);
        assert_eq!(header.extra_data.len(), 12);

        let inv_version = Header::try_from(invalid_version as &[u8]).unwrap_err();
        assert!(matches!(inv_version, HeaderDecodeError::UnsupportedVersion));

        let inv_magic = Header::try_from(invalid_magic as &[u8]).unwrap_err();
        assert!(matches!(inv_magic, HeaderDecodeError::InvalidMagic));

        let too_short = Header::try_from(too_short as &[u8]).unwrap_err();
        assert!(matches!(too_short, HeaderDecodeError::TooShort));

        let inv_padding = Header::try_from(invalid_padding as &[u8]).unwrap_err();
        assert!(matches!(inv_padding, HeaderDecodeError::InvalidPadding));
    }

    #[test]
    pub fn test_to_bytes() {
        let header = Header::new(1, Vec::new());
        let valid_header_exp = b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert_eq!(Vec::from(header).as_slice(), valid_header_exp);

        let header = Header::new(2, vec![1]);
        let valid_header_exp = b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x20\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert_eq!(Vec::from(header).as_slice(), valid_header_exp);

        let header = Header::new(2, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        let valid_header_exp = b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x20\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c";
        assert_eq!(Vec::from(header).as_slice(), valid_header_exp);

        let header = Header::new(2, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
        let valid_header_exp = b"CSGM\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x30\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert_eq!(Vec::from(header).as_slice(), valid_header_exp);
    }
}
