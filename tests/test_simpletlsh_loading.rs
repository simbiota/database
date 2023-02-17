use database::database::{Database, Object, ObjectImpl};
use database::formats::simple_tlsh::{SimpleTLSHEntryType, SimpleTLSHObject};
use database::raw_database_file::RawDatabaseFile;
use std::path::{Path, PathBuf};

#[test]
fn test_simpletlsh_load() {
    let file_data = std::fs::read(Path::new("test_files/generated1.sdb")).unwrap();
    let raw_database = RawDatabaseFile::try_from(file_data.as_slice()).unwrap();
    let raw_object = raw_database.objects.get(&1).unwrap();
    let object = Object::from(raw_object);
    let tlsh_object: SimpleTLSHObject = SimpleTLSHObject::from_object(object).unwrap();
    println!("{:#?}", tlsh_object.get_hashes());
}

#[test]
fn test_simpletlsh_saving() {
    let mut tlsh_object = SimpleTLSHObject::new(SimpleTLSHEntryType::HEX);
    tlsh_object.add_hash(
        "B911A8DACB5B5A06568B6ED299B18014C811DD897E95B720B871B1F5EF7300538187DC".to_string(),
    );
    tlsh_object.add_hash(
        "0B22C01977023F8A74C2CA8D7C4D514C426A3CB17C966FA2A0D96D770E7882C417FE5B".to_string(),
    );
    tlsh_object.add_hash(
        "79A31224C9D62CA19BDD6EAA5D43339038F85D8BF0932625D1D85A92EBBB3560FF41C0".to_string(),
    );
    tlsh_object.add_hash(
        "172533F8E717FDA43B4DD8F09E8A955912CB1DB6296DC0336E828B564C8260106FF16F".to_string(),
    );
    tlsh_object.add_hash(
        "3DB633814E9F2046252E5DD0E10FFBC4A54FEB96D02B4A158B33CE97B76888931937B7".to_string(),
    );
    let mut database = Database::new(1);
    database.add_object(1, tlsh_object.to_object());
    let bytes = database.as_bytes();
    std::fs::write(Path::new("test_files/generated1.sdb"), bytes.clone())
        .expect("failed to write file");
    let raw_db = RawDatabaseFile::try_from(bytes.as_slice()).expect("generated database invalid");
}
