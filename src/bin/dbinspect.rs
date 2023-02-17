extern crate core;

use clap::{value_parser, Arg, ArgAction, Command};
use database::database::{Object, ObjectImpl};
use database::formats;
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;

use database::header::Header;
use database::raw_database_file::RawDatabaseFile;

fn main() {
    let mut command = Command::new("dbinspect")
        .version("0.0.1")
        .author("Ukatemi Technologies Zrt.")
        .about("View SIMBIoTA database files")
        .arg(
            Arg::new("header")
                .short('d')
                .long("header")
                .help("Display file header information")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("extra-header")
                .long("extra")
                .help("Display extra header information")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("mapping")
                .short('m')
                .long("mapping")
                .help("Display object mappings")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("object-headers")
                .short('s')
                .long("object-headers")
                .help("Display object headers")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("database file")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        );
    let help_msg = command.render_long_help();
    let matches = command.get_matches();

    if !matches.get_flag("header") && !matches.get_flag("mapping") {
        println!("{}", help_msg);
        exit(1);
    }
    if matches.get_flag("extra-header") && !matches.get_flag("header") {
        println!("--extra requires --header");
        exit(1);
    }

    let file_path = matches.get_one::<PathBuf>("database file").unwrap();
    if !file_path.exists() {
        eprintln!("error: '{}': No such file", file_path.display());
        exit(1);
    }
    if !file_path.is_file() {
        eprintln!("error: '{}': Not a file", file_path.display());
        exit(1);
    }
    let mut file = std::fs::File::open(file_path).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    drop(file); // Force close it

    if matches.get_flag("header") {
        let header = Header::try_from(bytes.as_slice()).expect("invalid header");
        println!();
        println!("Header information:");
        println!("\tVersion: {0} ({0:#x})", header.version);
        println!(
            "\tNumber of formats: {0} ({0:#x})",
            header.number_of_objects
        );
        println!("\tHeader length: {0} ({0:#x})", header.header_len);

        if matches.get_flag("extra-header") {
            if header.version == 1 {
                if header.extra_data.len() < 16 {
                    println!("\tExtra header data(v1):\n\t\t<Invalid v1 header>");
                } else {
                    let timestamp =
                        u64::from_be_bytes((&header.extra_data[0..8]).try_into().unwrap());
                    let datetime =
                        chrono::NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap();
                    let version =
                        u64::from_be_bytes((&header.extra_data[8..16]).try_into().unwrap());
                    println!("\tExtra header data(v1):");
                    println!(
                        "\t\tModification timestamp: {} ({})",
                        timestamp,
                        datetime.format("%Y-%m-%d %H:%M:%S")
                    );
                    println!("\t\tDatabase version: {}", version);
                }
            } else {
                println!("No extra header information")
            }
        }
    }
    if matches.get_flag("mapping") {
        let (_, object_map) = RawDatabaseFile::debug_parse_v1_headers(bytes.as_slice())
            .expect("invalid database file");
        println!();
        println!("Object map:");
        println!("\t{:^16}   {:^16}", "ID", "Offset");
        for mapping in object_map.mappings {
            let is_invalid = mapping.offset as usize >= bytes.len();
            println!(
                "\t{:016x}   {:016x}{}",
                mapping.id,
                mapping.offset,
                if is_invalid { " - INVALID" } else { "" }
            );
        }
        println!();
    }
    if matches.get_flag("object-headers") {
        println!("Object headers:");
        let file = RawDatabaseFile::try_from(bytes.as_slice()).expect("invalid database file");
        for (id, object) in &file.objects {
            let parsed_object = formats::get_concrete_object(Object::from(object));
            println!("\tObject #{}", id);
            println!(
                "\t\tFormat: {:#x} ({})",
                object.format,
                if let Some(object_impl) = parsed_object {
                    get_object_name(object_impl)
                } else {
                    "unknown"
                }
            );
            println!(
                "\t\tCompression: {:#x} ({})",
                object.compression,
                get_compression_text(object.compression)
            );
            println!("\t\tEntry type: {:#x}", object.entry_type);
            println!("\t\tEntry size: {:#x} ({0:})", object.entry_size);
            println!("\t\tLength: {:#x} ({0:})", object.length);
            println!();
        }
    }
}

fn get_object_name<T>(_: T) -> &'static str
where
    T: ObjectImpl,
{
    T::NAME
}

fn get_compression_text(compression: u16) -> &'static str {
    match compression {
        0x0000 => "no compression",
        0x0001 => {
            if cfg!(feature = "compression") {
                "DEFLATE (flate2)"
            } else {
                "DEFLATE (not supported)"
            }
        }
        _ => "invalid/unknown",
    }
}
