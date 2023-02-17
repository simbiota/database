use num_integer::Integer;

pub mod database;
pub mod formats;

#[cfg(feature = "inspection")]
pub mod header;
#[cfg(not(feature = "inspection"))]
mod header;

#[cfg(feature = "inspection")]
pub mod object;
#[cfg(not(feature = "inspection"))]
mod object;

#[cfg(feature = "inspection")]
pub mod object_map;
#[cfg(not(feature = "inspection"))]
mod object_map;

#[cfg(feature = "inspection")]
pub mod raw_database_file;
#[cfg(not(feature = "inspection"))]
mod raw_database_file;

pub use database::{Database, LazyLoadedDatabase, LazyParsingError, Object, ObjectImpl};
pub use raw_database_file::DatabaseParseError;

pub(crate) fn next_multiple_of<T: Integer + Clone>(lhs: T, rhs: T) -> T {
    lhs.next_multiple_of(&rhs)
}
