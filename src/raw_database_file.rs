use crate::header::{Header, HeaderDecodeError};
use crate::object::{ObjectDecodeError, RawObject};
use crate::object_map::{ObjectMap, ObjectMapping, ObjectMappingError};
use crate::raw_database_file::DatabaseParseError::{
    InvalidHeader, InvalidObject, InvalidObjectMap, InvalidObjectOffset, UnsupportedVersion,
};
use std::collections::HashMap;

#[derive(Debug)]
pub enum DatabaseParseError {
    InvalidHeader(HeaderDecodeError),
    InvalidObjectMap(ObjectMappingError),
    InvalidObject(ObjectDecodeError),
    InvalidObjectOffset(ObjectMapping),
    UnsupportedVersion(u32),
    HeaderParsingError(&'static str),
    FileOpenFailed(std::io::Error),
    IOError(std::io::Error),
}
pub struct RawDatabaseFile {
    pub header: Header,
    pub object_map: ObjectMap,
    pub objects: HashMap<u64, RawObject>,
}

impl RawDatabaseFile {
    fn parse_v1(value: &[u8]) -> Result<Self, DatabaseParseError> {
        let (header, object_map) = Self::parse_v1_headers(value)?;

        // TODO: Implement lazy loading
        let objects = Self::parse_v1_objects(value, &object_map)?;

        Ok(Self {
            header,
            object_map,
            objects,
        })
    }

    fn parse_v1_headers(value: &[u8]) -> Result<(Header, ObjectMap), DatabaseParseError> {
        let header = Header::try_from(value).map_err(InvalidHeader)?;
        let header_size = header.header_len as usize;
        let remainig_bytes = &value[header_size..];

        let object_map = ObjectMap::try_from(remainig_bytes, header.number_of_objects)
            .map_err(InvalidObjectMap)?;
        Ok((header, object_map))
    }

    fn parse_v1_objects(
        data: &[u8],
        object_map: &ObjectMap,
    ) -> Result<HashMap<u64, RawObject>, DatabaseParseError> {
        let mut objects = HashMap::new();
        for mapping in &object_map.mappings {
            let start_pos = mapping.offset;
            if start_pos >= data.len() as u64 {
                return Err(InvalidObjectOffset(mapping.clone()));
            }

            let object_slice = &data[start_pos as usize..];
            let object = RawObject::try_from(object_slice).map_err(InvalidObject)?;
            objects.insert(mapping.id, object);
        }
        Ok(objects)
    }
}

impl TryFrom<&[u8]> for RawDatabaseFile {
    type Error = DatabaseParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let version = Header::partial_version(value).map_err(InvalidHeader)?;

        match version {
            0x0001 => Self::parse_v1(value),
            v => Err(UnsupportedVersion(v)),
        }
    }
}

#[cfg(feature = "inspection")]
impl RawDatabaseFile {
    /// Used for inspection feature to decode invalid databases with a valid header and object table
    pub fn debug_parse_v1_headers(value: &[u8]) -> Result<(Header, ObjectMap), DatabaseParseError> {
        RawDatabaseFile::parse_v1_headers(value)
    }
}
