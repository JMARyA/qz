use qz::read_archive;
use clap::{App, Arg};

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
                    Arg::with_name("target directory")
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
                )
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
        ("new", Some(_)) => {
            // TODO : Implement
        }
        ("test", Some(_)) => {
            // TODO : Implement
        }
        ("extract", Some(_)) => {
            // TODO : Implement
        }
        _ => {
            println!("{}", args.usage());
        }
    }
}
