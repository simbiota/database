# Database format for SIMBIoTA

Binary format, stores lists of data. Loosely based on git pack. Supports DEFLATE compression.

Note that database structure may change based on the feedback we receive and with new detectors.

## General information

This file format is designed to contain lists of entries of different types and formats. The file consists of a global 
header with general information about the file and a number of objects containing the actual data. Each object has an
ID number that can be used by the consumers to get the required object data.

The object mapping structure, part of the header, stores each object ID and the offset of the object within the file for
lazy loading. 

Each object starts with an object header with general information about the object, such as format and entry type. 
Compression can be enabled for objects that benefit from it, currently DEFLATE is supported. The object contains entries
in the entry type specified in the object header.

## File structure
```
+--------+----------------+----------+-----+----------+
| HEADER | OBJECT MAPPING | OBJECT 1 | ... | OBJECT N |
+--------+----------------+----------+-----+----------+
```
The database file contains the following sections:
- Header: Magic value, version and global metadata about the file
- Object mapping: The ID and offset of each object inside the file
- Objects: Object header and entries


### Global header
```
0                   4                   8                                      16  
+-------------------+-------------------+---------------------------------------+  
|       MAGIC       |      VERSION      |            NUMBER OF OBJECTS          |  
+-------------------+-------------------+-------------------+-------------------+  
|     HEADER LEN    |   Extra data 1    |   Extra data 2    |      Padding      |  
+-------------------+-------------------+---------------------------------------+  
```
- Magic: 4 byte magic: ascii 'CSGM'
- Version: 4 byte File format version, currently 1
- Number of objects: 8 byte number of objects stored in the file
- Header length: 4 byte length of the **whole header** (= 4 + 4 + 8 + 4 + x + PADDING)
- Extra data fields, defined in version
- Padding: Header must end 16 byte aligned

### Object mapping - info block for every object
```
0                                       8                                      16  
+---------------------------------------+---------------------------------------+  
|               OBJECT ID               |                 OFFSET                |  
+---------------------------------------+---------------------------------------+  
|                     // repeat this `number of objects` time                   |  
+---------------------------------------+---------------------------------------+  
```
- Object ID: 8 byte id for this object. Used by the detectors to find the related
data  
Note: The database can have objects with the same id, the data will be concatenated together
- Offset: 8 byte offset of the object header from the beginning of the file

### One object
```
0             2             4             6             8                                                      16  
+-------------+-------------+-------------+-------------+-------------------------------------------------------+
|   FORMAT    | COMPRESSION | ENTRY TYPE  | ENTRY SIZE  |                        LENGTH                         |  
+-------------+-------------+-------------+-------------+-------------------------------------------------------+
|                       ENTRY 1                       |                       ENTRY 2                       |   +  
                    ENTRY 3                       |                       ...                       |  PADDING  |  
+---------------------------------------------------------------------------------------------------------------+  
```
- Format: 2 byte decoding format of the data, formats defined below
- Compression: 2 byte compression format, described below
- Entry type: 2 byte type of each entry, can be used for updating the object format
- Entry size: 2 byte size of each entry
- Length: 8 byte size of the whole object (header + entries), _without the padding_
- Padding: Padded to 16 byte alignment


- The entries may be compressed or laid out tightly packed

- Padding: Object must end 16 byte aligned

## Version 1

- Version number: 1
- Extra fields:
    - Last updated: 64bit unix timestamp
    - Database version: 64bit version of the current dataset

The Version 1 header contains the following extra data structure:
```
+-------------------+-------------------+-------------------+-------------------+  
|                                      ...                                      |
+-------------------+-------------------+-------------------+-------------------+  
|     HEADER LEN    |           Last update time            |  DB version[0:4]  |  
+-------------------+-------------------+---------------------------------------+ 
|  DB version[4:8]  |                          Padding                          |
+-------------------+-------------------+---------------------------------------+
```

### Formats

The following formats are supported:

- 0x0001: Simple TLSH:  
Raw 35 byte TLSH hashes  
  Entry types:
  - 0x0: HEX format TLSH (70 bytes)
  - 0x1: binary tlsh (35 bytes)
        
- 0x0002: TLSH database:  
Database for our own TLSH hash variants and the corresponding sample SHA256  
Entry types:
  - 0x0: Binary TLSH digest and a SHA256 hash

- 0x0003: TLSH database with per-sample distance:
Extend the 0x0002 database type with an additional distance byte per sample. This
is used for more fine-tuned malware detection.
Entry types:
  - 0x0: Binary TLSH digest, SHA256 hash and distance byte

### Compression:

The following compression values are supported:
- 0x0000: No compression, each entry is placed after each other tightly packed
- 0x0001: DEFLATE compression, with the [`flate2`](https://crates.io/crates/flate2) crates default settings
