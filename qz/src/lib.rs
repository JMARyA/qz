use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::Write;

pub mod errors;

//   -----------
//   | STRUCTS |
//   -----------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CompressionAlgo {
    ZSTD,
    LZ4,
    NONE,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QZFile {
    pub name: String,
    pub compression: CompressionAlgo,
    pub checksum: u32,
    index_start: u64,
    pub index_size: u64,
}

impl QZFile {
    // Return file data from archive with header offset
    fn read_file(&self, archive: &str, offset: u64) -> Result<Vec<u8>, errors::FileReadError> {
        let mut f = File::open(archive).unwrap();
        let mut read_buf: Vec<u8> = vec![0u8; self.index_size as usize];
        f.seek(std::io::SeekFrom::Start(offset + self.index_start))
            .unwrap();
        let res = f.read_exact(&mut read_buf);
        if res.is_err() {
            return Err(errors::FileReadError::Other(format!(
                "{:?}",
                res.unwrap_err()
            )));
        }

        //println!("reading {:?}", self);

        // CHECKSUM

        let hash = crc32fast::hash(&read_buf);
        if hash != self.checksum {
            return Err(errors::FileReadError::Checksum(hash, self.checksum));
        }

        // COMPRESSION

        match self.compression {
            CompressionAlgo::ZSTD => {
                let res = zstd::stream::decode_all(&read_buf[0..read_buf.len()]);
                if res.is_err() {
                    return Err(errors::FileReadError::CompressionError);
                }
                read_buf = res.unwrap();
            }
            CompressionAlgo::LZ4 => {
                let res = lz4_compression::decompress::decompress(&read_buf);
                if res.is_err() {
                    return Err(errors::FileReadError::CompressionError);
                }
                read_buf = res.unwrap();
            }
            CompressionAlgo::NONE => {}
        }

        Ok(read_buf)
    }

    fn is_valid(&self, archive: &str, offset: u64) -> Result<(), errors::FileReadError> {
        let res = self.read_file(archive, offset);
        match res {
            Ok(_) => Ok(()),
            Err(errors::FileReadError::Checksum(real, exp)) => {
                Err(errors::FileReadError::Checksum(real, exp))
            }
            _ => Err(errors::FileReadError::Other(String::new())),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QZDir {
    pub name: String,
    pub content: Vec<QZEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QZEntry {
    Dir(QZDir),
    File(QZFile),
}

/// Header for QZ Archive
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QZArchiveHeader {
    pub name: String,
    pub info: String,
    pub version: String,
    root: QZEntry,
}

// Turn directory structure into QZEntry structure
fn pack_dir(dir: &str, compression: CompressionAlgo) -> QZEntry {
    let mut content: Vec<QZEntry> = vec![];

    let paths = fs::read_dir(dir).unwrap();

    for p in paths {
        let p = p.unwrap();
        //println!("Scanning {}", p.path().display());
        if p.metadata().unwrap().is_file() {
            let f = QZFile {
                name: String::from(p.path().file_name().unwrap().to_str().unwrap()),
                compression: compression.clone(),
                checksum: 0,
                index_start: 0,
                index_size: 0,
            };
            content.push(QZEntry::File(f));
        } else if p.metadata().unwrap().is_dir() {
            let d = pack_dir(p.path().to_str().unwrap(), compression.clone());
            content.push(d);
        }
    }

    return QZEntry::Dir(QZDir {
        name: std::path::Path::new(dir)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        content,
    });
}

//   ---------
//   | WRITE |
//   ---------

/// Creating a QZ Archive
pub fn create_archive(
    dir: &str,
    out_file: &str,
    name: &str,
    description: &str,
    compression: CompressionAlgo,
) {
    // SCAN DIR
    let mut root = pack_dir(dir, compression);

    // PROCESS & MAKE FILE

    let mut files_content: Vec<u8> = vec![];

    fn write_files_dir(d: &mut QZDir, path: &str, mut f_content: Vec<u8>) -> (Vec<u8>,) {
        for e in &mut d.content {
            match e {
                QZEntry::Dir(ref mut d) => {
                    // RECURSIVE
                    let path = std::path::Path::new(path).join(&d.name);
                    let path = path.to_str().unwrap();
                    let res = write_files_dir(d, path, f_content);
                    f_content = res.0;
                }
                QZEntry::File(ref mut f) => {
                    let path = std::path::Path::new(path).join(&f.name);
                    //println!("p {}", path.to_str().unwrap());
                    println!("Adding file {:?}", &f);
                    f.index_start = f_content.len() as u64;

                    let mut file = std::fs::File::open(&path).expect("no file found");
                    let metadata = fs::metadata(&path).expect("unable to read metadata");
                    let mut buffer = vec![0; metadata.len() as usize];
                    file.read_exact(&mut buffer).expect("buffer overflow");

                    // COMPRESSION

                    match f.compression {
                        CompressionAlgo::ZSTD => {
                            buffer = zstd::stream::encode_all(&buffer[0..buffer.len()], 5).unwrap();
                        }
                        CompressionAlgo::LZ4 => {
                            buffer = lz4_compression::compress::compress(&buffer);
                        }
                        CompressionAlgo::NONE => {}
                    }

                    // CHECKSUM

                    f.checksum = crc32fast::hash(&buffer);

                    f.index_size = buffer.len() as u64;

                    f_content.extend(buffer);
                }
            }
        }
        (f_content,)
    }

    if let QZEntry::Dir(ref mut d) = root {
        let res = write_files_dir(d, dir, files_content);
        files_content = res.0;
    }

    let archive = QZArchiveHeader {
        name: name.to_string(),
        info: description.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        root,
    };

    let mut header = serde_json::to_vec(&archive).unwrap();

    // COMPRESSION
    header = zstd::stream::encode_all(&header[0..header.len()], 5).unwrap();

    let header_size = header.len().to_ne_bytes();

    // SAVE
    fs::File::create(out_file).unwrap();
    let mut final_archive = fs::OpenOptions::new()
        .write(true)
        .append(true) // This is needed to append to file
        .open(out_file)
        .unwrap();

    final_archive.write_all(&header_size).unwrap();
    final_archive.write_all(&header).unwrap();
    final_archive.write_all(&files_content).unwrap();
}

//   --------
//   | READ |
//   --------

/// Struct for handling QZ Archives
pub struct QZArchive {
    archive_file: String,
    header_size: u64,
    pub header: QZArchiveHeader,
}

impl QZArchive {
    /// Reading a file from archive
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, errors::FileReadError> {
        let path = QZArchive::get_path(path);
        let mut path_c = std::path::Path::new(&path).components();

        if path_c.next() == Some(std::path::Component::RootDir) {
            let res = QZArchive::_get_entry(path_c, &self.header.root);
            if res.is_err() {
                return Err(errors::FileReadError::Other(format!(
                    "{:?}",
                    res.unwrap_err()
                )));
            }
            let res = res.unwrap();

            match res {
                QZEntry::Dir(_) => {
                    return Err(errors::FileReadError::NotAFile);
                }
                QZEntry::File(f) => {
                    return f.read_file(&self.archive_file, self.header_size + 8);
                }
            }
        }

        Err(errors::FileReadError::NotFound)
    }

    pub fn check_file(&self, path: &str) -> Result<(), errors::FileReadError> {
        let path = QZArchive::get_path(path);
        let mut path_c = std::path::Path::new(&path).components();

        if path_c.next() == Some(std::path::Component::RootDir) {
            let res = QZArchive::_get_entry(path_c, &self.header.root);
            if res.is_err() {
                return Err(errors::FileReadError::Other(format!(
                    "{:?}",
                    res.unwrap_err()
                )));
            }
            let res = res.unwrap();

            match res {
                QZEntry::Dir(_) => {
                    return Err(errors::FileReadError::NotAFile);
                }
                QZEntry::File(f) => {
                    f.is_valid(&self.archive_file, self.header_size + 8)?;
                    return Ok(());
                }
            }
        }
        Err(errors::FileReadError::NotFound)
    }

    fn get_path(path: &str) -> String {
        format!("/{path}")
    }

    /// Get qz entry for given path
    pub fn get_entry(&self, path: &str) -> Result<QZEntry, errors::EntryError> {
        let path = QZArchive::get_path(path);
        let mut path_c = std::path::Path::new(&path).components();

        if path_c.next() == Some(std::path::Component::RootDir) {
            let res = QZArchive::_get_entry(path_c, &self.header.root);
            return res;
        }
        Err(errors::EntryError::Other(String::new()))
    }

    fn _get_entry(
        mut comp: std::path::Components,
        current_entry: &QZEntry,
    ) -> Result<QZEntry, errors::EntryError> {
        if let QZEntry::Dir(current_dir) = current_entry {
            match comp.next() {
                Some(std::path::Component::Normal(walk_path_name)) => {
                    for e in &current_dir.content {
                        match e {
                            QZEntry::Dir(d) => {
                                //println!("matching {:?} and {:?}", d.name, walk_path_name.to_str().unwrap());
                                if d.name == walk_path_name.to_str().unwrap() {
                                    return QZArchive::_get_entry(comp, &QZEntry::Dir(d.clone()));
                                }
                            }
                            QZEntry::File(f) => {
                                if f.name == walk_path_name.to_str().unwrap() {
                                    return Ok(QZEntry::File(f.clone()));
                                }
                            }
                        }
                    }
                    return Err(errors::EntryError::NothingFound);
                }
                None => {
                    return Ok(current_entry.clone());
                }
                _ => {
                    return Err(errors::EntryError::PathError);
                }
            }
        }
        Err(errors::EntryError::Other(String::new()))
    }

    /// List content of directory returning list with filenames
    pub fn ls(&self, path: &str) -> Result<Vec<String>, errors::ListingError> {
        let path = QZArchive::get_path(path);
        let mut path_c = std::path::Path::new(&path).components();

        let mut content: Vec<String> = vec![];

        if path_c.next() == Some(std::path::Component::RootDir) {
            let res = QZArchive::_get_entry(path_c, &self.header.root);
            if res.is_err() {
                return Err(errors::ListingError::Other(format!(
                    "{:?}",
                    res.unwrap_err()
                )));
            }
            let res = res.unwrap();

            match res {
                QZEntry::Dir(d) => {
                    for e in d.content {
                        match e {
                            QZEntry::Dir(d) => {
                                content.push(d.name);
                            }
                            QZEntry::File(f) => {
                                content.push(f.name);
                            }
                        }
                    }
                }
                _ => {
                    return Err(errors::ListingError::IsFile);
                }
            }
        }

        Ok(content)
    }
}

/// Read Archive File and return a QZArchive Struct
pub fn read_archive(path: &str) -> Result<QZArchive, errors::ReadError> {
    // OPEN FILE
    let f = File::open(path);
    if f.is_err() {
        return Err(errors::ReadError::new("failed to open archive file"));
    }
    let mut f = f.unwrap();

    // GET HEADER
    let mut size_buf: [u8; 8] = [0; 8];
    let err = f.read_exact(&mut size_buf);
    if err.is_err() {
        return Err(errors::ReadError::new("failed to read header size"));
    }
    let size = u64::from_ne_bytes(size_buf);
    //println!("size {}", size);

    // READ HEADER
    let mut header_buf: Vec<u8> = vec![0u8; size as usize];
    f.seek(std::io::SeekFrom::Start(8)).unwrap();
    let err = f.read_exact(&mut header_buf);
    if err.is_err() {
        return Err(errors::ReadError::new("failed to read header"));
    }
    let res = zstd::stream::decode_all(&header_buf[0..header_buf.len()]);
    if res.is_err() {
        return Err(errors::ReadError::new("failed to decompress header"));
    }
    header_buf = res.unwrap();

    // DESERIALIZE
    let header: Result<QZArchiveHeader, _> = serde_json::from_slice(&header_buf);
    if header.is_err() {
        return Err(errors::ReadError::new("failed to decode header"));
    }
    let header = header.unwrap();

    //println!("header {:?}", header);

    Ok(QZArchive {
        archive_file: path.to_string(),
        header_size: size,
        header,
    })
}
