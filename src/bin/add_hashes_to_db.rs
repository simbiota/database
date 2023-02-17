use simbiota_database::formats::simple_tlsh::SimpleTLSHObject;
use simbiota_database::{Database, ObjectImpl};
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let dbfile = &args[1];
    let db_data = std::fs::read(dbfile).unwrap();

    let mut raw_database = Database::from_bytes(db_data.as_slice()).unwrap();

    let obj = raw_database.get_object(0x0001).unwrap();
    let mut tlsh_obj = SimpleTLSHObject::from_object(obj.clone()).unwrap();

    let mut stdin_lines = String::new();
    std::io::stdin().read_to_string(&mut stdin_lines).unwrap();

    for line in stdin_lines.split('\n') {
        if line.len() != 70 {
            eprintln!("invalid TLSH hash: {}", line);
            continue;
        }

        tlsh_obj.add_hash(line.to_owned());
    }

    raw_database.add_object(0x0001, tlsh_obj.to_object());

    let bytes = raw_database.as_bytes();
    std::fs::write(dbfile, bytes).unwrap();
}
