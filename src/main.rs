use clap::{App, Arg};
use qz::read_archive;

fn main() {
    let args = App::new("QZip")
        .version(option_env!("CARGO_PKG_VERSION").unwrap())
        .author("JMARyA <jmarya0@icloud.com>")
        .about("QZip Format")
        .subcommand(
            App::new("new")
                .about("create new .qz file")
                .arg(
                    Arg::with_name("archive")
                        .required(true)
                        .value_name("ARCHIVE")
                        .help("Archive Filename"),
                )
                .arg(
                    Arg::with_name("target")
                        .required(true)
                        .value_name("TARGET")
                        .help("directory to pack"),
                ),
        )
        .subcommand(
            App::new("ls")
                .about("list contents of .qz file")
                .arg(
                    Arg::with_name("archive")
                        .required(true)
                        .value_name("ARCHIVE")
                        .help("Archive File"),
                )
                .arg(
                    Arg::with_name("path")
                        .help("list files at specified path")
                        .default_value("/")
                        .value_name("PATH")
                        .required(false),
                ),
        )
        .subcommand(
            App::new("extract")
                .about("extract a .qz file")
                .arg(
                    Arg::with_name("archive")
                        .required(true)
                        .value_name("ARCHIVE")
                        .help("Archive Filename"),
                )
                .arg(
                    Arg::with_name("target directory")
                        .short("d")
                        .long("destination")
                        .required(false)
                        .value_name("DESTINATION")
                        .help("directory to unpack"),
                )
                .arg(
                    Arg::with_name("path")
                        .required(false)
                        .value_name("PATH")
                        .help("specific path to be unpacked"),
                ),
        )
        .subcommand(
            App::new("info").about("list archive info").arg(
                Arg::with_name("archive")
                    .required(true)
                    .value_name("ARCHIVE")
                    .help("Archive File"),
            ),
        )
        .subcommand(
            App::new("test").about("test archive integrity").arg(
                Arg::with_name("archive")
                    .required(true)
                    .value_name("ARCHIVE")
                    .help("Archive File"),
            ),
        )
        .get_matches();

    match args.subcommand() {
        ("info", Some(cmd)) => {
            let archive_file = cmd.value_of("archive").unwrap();
            let a = read_archive(archive_file).unwrap();
            println!("QZ Archive v.{}: \'{}\'", &a.header.version, &a.header.name);
            println!("{}", &a.header.info);
        }
        ("ls", Some(cmd)) => {
            let archive_file = cmd.value_of("archive").unwrap();
            let path = format!("/{}", cmd.value_of("path").unwrap());
            let path = path.replace("//", "/");
            let a = read_archive(archive_file).unwrap();
            println!("QZ Archive \'{}\' : {}", &a.header.name, &path);
            let dir_content = a.ls(&path).unwrap();
            for f in dir_content {
                println!("{}", std::path::Path::new(&path).join(f).to_str().unwrap());
            }
        }
        ("new", Some(cmd)) => {
            // TODO : Implement
            let archive_file = cmd.value_of("archive").unwrap();
            let target = cmd.value_of("target").unwrap();

            qz::create_archive(&target, &archive_file);
        }
        ("test", Some(cmd)) => {
            let archive_file = cmd.value_of("archive").unwrap();
            let a = read_archive(archive_file).unwrap();

            fn check_recursive(a: &qz::QZArchive, path: &str) {
                //println!("checking path {}", &path);
                let dir_content = a.ls(path).unwrap();
                for f in dir_content {
                    let entry = a.get_entry(std::path::Path::new(path).join(f).to_str().unwrap()).unwrap();
                    match entry {
                        qz::QZEntry::Dir(d) => {
                            check_recursive(a, std::path::Path::new(path).join(&d.name).to_str().unwrap());
                        },
                        qz::QZEntry::File(file) => {
                            let f_path = std::path::Path::new(path).join(&file.name);
                            //println!("checking file path {}", f_path.to_str().unwrap());
                            let res = a.check_file(f_path.to_str().unwrap());
                            if res.is_err() {
                                let err = res.unwrap_err();
                                match err {
                                    qz::errors::FileReadError::Checksum(real, exp) => {
                                println!("Error checking archive: Damaged file {} (Expected Checksum {} but got {})", f_path.to_str().unwrap(), exp, real);
                                    }
                                    _ => {} 
                                }
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
            
            check_recursive(&a, "/");
            println!("Everything ok")
        }
        ("extract", Some(_)) => {
            // TODO : Implement
        }
        _ => {
            println!("{}", args.usage());
        }
    }
}
