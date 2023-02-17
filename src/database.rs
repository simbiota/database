//! High-level implementation of the database system. Use these structs and functions
//! for normal database manipulation.
//!
//! # Example: Load hashes from a database file
//! ```rust no_run
//! use std::path::Path;
//! use database::database::{Database, ObjectImpl};
//! use database::formats;
//! use formats::simple_tlsh::SimpleTLSHObject;
//! use simbiota_database::{Database, ObjectImpl};
//! use simbiota_database::formats::simple_tlsh::SimpleTLSHObject;
//!
//! let file_data = std::fs::read(Path::new("database_file.sdb")).unwrap();
//! let database = Database::from_bytes(file_data.as_slice()).expect("failed to load database");
//! let object = database.get_object(0x0001).expect("object not found");
//! let tlsh_list = SimpleTLSHObject::from_object(object.clone()).expect("failed to parse object");
//! println!("Entries: {}", tlsh_list.get_hashes());
//! ```
//!
//! # Example: Create a new database and save it to a file
//! ```rust no_run
//! use std::path::Path;
//! use database::database::{Database, ObjectImpl};
//! use database::formats::simple_tlsh::{SimpleTLSHEntryType, SimpleTLSHObject};
//! use simbiota_database::{Database, ObjectImpl};
//! use simbiota_database::formats::simple_tlsh::{SimpleTLSHEntryType, SimpleTLSHObject};
//!
//! let mut tlsh_object = SimpleTLSHObject::new(SimpleTLSHEntryType::HEX);
//!     tlsh_object.add_hash(
//!         "B911A8DACB5B5A06568B6ED299B18014C811DD897E95B720B871B1F5EF7300538187DC".to_string(),
//!     );
//!     tlsh_object.add_hash(
//!         "0B22C01977023F8A74C2CA8D7C4D514C426A3CB17C966FA2A0D96D770E7882C417FE5B".to_string(),
//!     );
//!     tlsh_object.add_hash(
//!         "79A31224C9D62CA19BDD6EAA5D43339038F85D8BF0932625D1D85A92EBBB3560FF41C0".to_string(),
//!     );
//!     tlsh_object.add_hash(
//!         "172533F8E717FDA43B4DD8F09E8A955912CB1DB6296DC0336E828B564C8260106FF16F".to_string(),
//!     );
//!     tlsh_object.add_hash(
//!         "3DB633814E9F2046252E5DD0E10FFBC4A54FEB96D02B4A158B33CE97B76888931937B7".to_string(),
//!     );
//!     let mut database = Database::new(1);
//!     database.add_object(1, tlsh_object.to_object());
//!     let bytes = database.as_bytes();
//!     std::fs::write(Path::new("test_files/generated1.sdb"), bytes.clone())
//!         .expect("failed to write file");
//! ```

use crate::database::LazyParsingError::{InvalidObject, NotFound};
use crate::database::ObjectCompressionType::{NoCompression, DEFLATE};
use crate::header::Header;
use crate::object::{ObjectDecodeError, RawObject};
use crate::object_map::{ObjectMap, ObjectMapping};
use crate::raw_database_file::DatabaseParseError::{
    FileOpenFailed, HeaderParsingError, IOError, InvalidHeader, InvalidObjectMap,
};
use crate::raw_database_file::{DatabaseParseError, RawDatabaseFile};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
#[cfg(target_family = "unix")]
use std::os::unix::fs::FileExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::FileExt;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Compression type setting for objects.
#[derive(Clone)]
pub enum ObjectCompressionType {
    /// Entries are not compressed.
    NoCompression,
    /// Using DEFLATE compression, the entries are placed after each other.
    /// and then compressed using `flate2`'s default compressor.
    DEFLATE,
}

impl ObjectCompressionType {
    pub fn get_value(&self) -> u16 {
        match self {
            NoCompression => 0x0000,
            DEFLATE => 0x0001,
        }
    }

    pub fn from_value(value: u16) -> Self {
        match value {
            0x0000 => NoCompression,
            0x0001 => DEFLATE,
            _ => panic!("invalid compression type"),
        }
    }
}

/// Trait for all object types.
/// Each ObjectImpl represents an object formats and provides the implementation for interpreting
/// the entries inside the object
pub trait ObjectImpl: Sized {
    /// Name of the object format. Used for debugging purposes.
    const NAME: &'static str;
    /// Create a generic object from the concrete instance to be saved into the database.
    fn to_object(self) -> Object;
    /// Parse the data from a generic object. The implementation must check whether the generic
    /// object's format and entry type is valid.
    ///
    /// Returns none if the object is not in the implementation's format.
    fn from_object(obj: Object) -> Option<Self>
    where
        Self: Sized;
}

/// Generic database object.
#[derive(Clone)]
pub struct Object {
    /// Format of this object. Used for decoding the data to a specific [`ObjectImpl`].
    pub(crate) format: u16,
    /// Compression settings for this object.
    pub(crate) compression_type: ObjectCompressionType,
    /// What kind of entries are stored inside this object. One format can support multiple
    /// entry types (e.g hex and binary hashes).
    pub(crate) entry_type: u16,
    /// Size of each entry, used for decoding.
    pub(crate) entry_size: u16,
    /// Raw data of each entry.
    pub(crate) data: Vec<Vec<u8>>,
}

impl From<&RawObject> for Object {
    /// Create a [`Object`] from a [`RawObject`] reference, _copying_ the data.
    fn from(value: &RawObject) -> Self {
        Self {
            format: value.format,
            compression_type: ObjectCompressionType::from_value(value.compression),
            entry_type: value.entry_type,
            entry_size: value.entry_size,
            data: value.data.clone(),
        }
    }
}

impl From<RawObject> for Object {
    /// Create a [`Object`] from a [`RawObject`] reference, consuming it and reusing the data.
    fn from(value: RawObject) -> Self {
        Self {
            format: value.format,
            compression_type: ObjectCompressionType::from_value(value.compression),
            entry_type: value.entry_type,
            entry_size: value.entry_size,
            data: value.data,
        }
    }
}

/// Error representing failures that can occur in a [`LazyLoadedDatabase`]
#[derive(Debug)]
pub enum LazyParsingError {
    IOError(std::io::Error),
    NotFound,
    InvalidObject(ObjectDecodeError),
}

/// A special database instance designed for low-memory applications. It does not load and store the
/// whole database file into memory, only the minial header information.
///
/// Objects can be read lazily, only the required parts will be in memory.
/// For better access time, use [`Database`].
pub struct LazyLoadedDatabase {
    file: File,
    _header: Header,
    mapping: ObjectMap,
}

impl LazyLoadedDatabase {
    /// Explicitly close the database
    pub fn close(self) {
        // noop
    }

    /// Create a new [`LazyLoadedDatabase`] from a specified file path. The header and object map
    /// are loaded and kept in memory, but no objects are loaded.
    pub fn new(file: &Path) -> Result<Self, DatabaseParseError> {
        let mut file = std::fs::File::open(file).map_err(FileOpenFailed)?;

        // Read minimal header
        let mut minimal_header_buf = [0u8; 0x20];
        file.read_exact(&mut minimal_header_buf).map_err(IOError)?;
        let length = u32::from_be_bytes((&minimal_header_buf[16..20]).try_into().unwrap());
        let mut header_data = Vec::with_capacity(length as usize);
        read_exact_offset(&file, header_data.as_mut_slice(), 0).map_err(IOError)?;

        let header = Header::try_from(header_data.as_slice()).map_err(InvalidHeader)?;
        let mapping_size = 16 * header.number_of_objects;
        let mut mapping_data = Vec::with_capacity(mapping_size as usize);
        read_exact_offset(&file, mapping_data.as_mut_slice(), header.header_len as u64)
            .map_err(IOError)?;
        let mapping = ObjectMap::try_from(mapping_data.as_slice(), header.number_of_objects)
            .map_err(InvalidObjectMap)?;
        Ok(Self {
            file,
            _header: header,
            mapping,
        })
    }

    /// Check if the database contains a specified object.
    pub fn has_object(&self, id: u64) -> bool {
        self.mapping.mappings.iter().any(|m| m.id == id)
    }

    /// Reads the requested object from the database if possible.
    ///
    /// Note: Requesting the same object multiple times results in reading and interpreting the
    /// data each time.
    pub fn get_object(&self, id: u64) -> Result<Object, LazyParsingError> {
        if !self.has_object(id) {
            return Err(NotFound);
        }
        let mapping = self.mapping.mappings.iter().find(|m| m.id == id).unwrap();
        let mut temp_obj_header = [0u8; 16];
        read_exact_offset(&self.file, &mut temp_obj_header, mapping.offset)
            .map_err(LazyParsingError::IOError)?;

        let len = u64::from_be_bytes((&temp_obj_header[8..16]).try_into().unwrap());
        let mut object_data = Vec::with_capacity(len as usize);
        read_exact_offset(&self.file, object_data.as_mut_slice(), mapping.offset)
            .map_err(LazyParsingError::IOError)?;

        let raw_object = RawObject::try_from(object_data)
            .map_err(InvalidObject)
            .unwrap();
        let object = Object::from(raw_object);
        Ok(object)
    }
}

/// High-level interface for a database.
///
/// The database information and all objects in it are kept in memory for faster access.
/// For resource-constrained environments, use [`LazyLoadedDatabase`].
pub struct Database {
    objects: HashMap<u64, Object>,
    _last_updated: u64,
    database_version: u64,
}

impl Database {
    /// Create a new, empty database with the provided v1 version number.
    pub fn new(database_version: u64) -> Self {
        Database {
            objects: HashMap::new(),
            _last_updated: 0,
            database_version,
        }
    }

    /// Add an object with the specified id to the database.
    ///
    /// Note: Adding multiple objects with the same ID is currently not
    /// supported and results in a panic.
    pub fn add_object(&mut self, id: u64, obj: Object) {
        // TODO: Merge objects
        self.objects.insert(id, obj);
    }

    /// Get a stored object from the database by its ID.
    pub fn get_object(&self, id: u64) -> Option<&Object> {
        self.objects.get(&id)
    }

    /// Get a mutable stored object from the database by its ID
    pub fn get_object_mut(&mut self, id: u64) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }

    /// Loads the database from a byte stream.
    ///
    /// Parses the header and loads all objects into memory.
    pub fn from_bytes(data: &[u8]) -> Result<Self, DatabaseParseError> {
        let raw_database = RawDatabaseFile::try_from(data)?;
        let extra_data = &raw_database.header.extra_data;

        if extra_data.len() < 16 {
            return Err(HeaderParsingError("missing v1 extra data"));
        }

        let timestamp_bytes = &extra_data[0..8];
        let version_bytes = &extra_data[8..16];

        let timestamp = u64::from_be_bytes(timestamp_bytes.try_into().unwrap());
        let version = u64::from_be_bytes(version_bytes.try_into().unwrap());

        let mut objects = HashMap::new();
        for (id, raw_obj) in raw_database.objects.iter() {
            let obj = Object::from(raw_obj);
            objects.insert(*id, obj);
        }

        Ok(Self {
            objects,
            _last_updated: timestamp,
            database_version: version,
        })
    }

    /// Serialize the database to binary format. Uses the current system time
    /// for the modification date.
    pub fn as_bytes(&self) -> Vec<u8> {
        let timestamp: u64 = (std::time::SystemTime::now().duration_since(UNIX_EPOCH))
            .unwrap()
            .as_secs();
        let extra_data = {
            let mut data = Vec::new();
            timestamp.to_be_bytes().iter().for_each(|v| data.push(*v));
            self.database_version
                .to_be_bytes()
                .iter()
                .for_each(|v| data.push(*v));
            data
        };
        let header = Header::new(self.objects.len() as u64, extra_data);
        let mut output_data = Vec::from(header);
        let header_len = output_data.len();
        let mut mappings: Vec<ObjectMapping> = Vec::new();
        let mut object_data = Vec::new();

        for (id, object) in &self.objects {
            let mut raw_object = RawObject::new(
                object.format,
                object.compression_type.get_value(),
                object.entry_type,
                object.entry_size,
            );
            raw_object.data = object.data.clone();
            let pre_offset = object_data.len();
            if pre_offset & 16 != 0 {
                panic!("someone f-d up the padding");
            }
            mappings.push(ObjectMapping::new(*id, pre_offset as u64));
            let mut out_vec = Vec::from(raw_object);
            object_data.append(&mut out_vec);
        }

        // Patch mappings
        let offset = header_len as u64 + (16 * mappings.len()) as u64;
        for mapping in mappings.iter_mut() {
            mapping.offset += offset;
        }

        let mut object_map = ObjectMap::new();
        object_map.mappings = mappings;
        let mut mapping_vec = Vec::from(object_map);
        output_data.append(&mut mapping_vec);
        output_data.append(&mut object_data);
        output_data
    }
}

fn read_exact_offset(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    #[cfg(target_family = "unix")]
    return file.read_exact_at(buf, offset);
    #[cfg(target_os = "windows")]
    return file.seek_read(buf, offset).map(|s| {});
}
