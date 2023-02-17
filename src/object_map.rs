use crate::object_map::ObjectMappingError::{InvalidLength, InvalidPadding};

#[derive(Debug)]
pub enum ObjectMappingError {
    InvalidLength,
    InvalidPadding,
}

#[derive(Debug, Clone)]
pub struct ObjectMapping {
    pub id: u64,
    pub offset: u64,
}

impl ObjectMapping {
    pub fn new(id: u64, offset: u64) -> Self {
        Self { id, offset }
    }
}

impl TryFrom<&[u8]> for ObjectMapping {
    type Error = ObjectMappingError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 16 {
            return Err(InvalidLength);
        }

        let id = u64::from_be_bytes((&value[0..8]).try_into().unwrap());
        let offset = u64::from_be_bytes((&value[8..16]).try_into().unwrap());
        if offset % 16 != 0 {
            return Err(InvalidPadding);
        }

        Ok(Self { id, offset })
    }
}

#[derive(Debug)]
pub struct ObjectMap {
    pub mappings: Vec<ObjectMapping>,
}

impl ObjectMap {
    pub(crate) fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    pub(crate) fn try_from(value: &[u8], entry_count: u64) -> Result<Self, ObjectMappingError> {
        let data_len = value.len();
        if data_len == 0 {
            return Err(InvalidLength);
        }

        if data_len < (entry_count * 16) as usize {
            return Err(InvalidLength);
        }

        let num_of_entries = entry_count;

        let mut entries = Vec::new();
        for index in 0..num_of_entries {
            let offset = (index * 16) as usize;
            let mapping = ObjectMapping::try_from(&value[offset..offset + 16])?;
            entries.push(mapping);
        }

        Ok(Self { mappings: entries })
    }
}

impl From<ObjectMap> for Vec<u8> {
    fn from(value: ObjectMap) -> Self {
        let mut data = Vec::new();

        for obj in &value.mappings {
            obj.id.to_be_bytes().iter().for_each(|v| data.push(*v));
            obj.offset.to_be_bytes().iter().for_each(|v| data.push(*v));
        }

        data
    }
}

#[cfg(test)]
mod test {
    use crate::object_map::{ObjectMap, ObjectMapping, ObjectMappingError};

    #[test]
    pub fn test_mapping_load() {
        let raw_data = b"\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x55\xaa";
        let invalid_len = b"\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x55";
        let mapping = ObjectMap::try_from(raw_data as &[u8], 2).unwrap();
        assert_eq!(mapping.mappings.len(), 2);
        assert_eq!(mapping.mappings[0].id, 1);
        assert_eq!(mapping.mappings[1].id, 32);
        assert_eq!(mapping.mappings[0].offset, 32);
        assert_eq!(mapping.mappings[1].offset, 0x55aa);

        let invalid_err = ObjectMap::try_from(invalid_len as &[u8], 2).unwrap_err();
        assert!(matches!(invalid_err, ObjectMappingError::InvalidLength));
    }

    #[test]
    pub fn test_mapping_store() {
        let raw_data = b"\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x55\xaa";
        let mut mapping = ObjectMap::new();
        mapping.mappings.push(ObjectMapping { id: 1, offset: 32 });
        mapping.mappings.push(ObjectMapping {
            id: 32,
            offset: 0x55aa,
        });

        assert_eq!(Vec::from(mapping).as_slice(), raw_data);
    }
}
