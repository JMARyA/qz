#[derive(Debug)]
pub struct ReadError {
    msg: String,
}

impl ReadError {
    pub fn new(msg: &str) -> ReadError {
        return ReadError {
            msg: msg.to_string(),
        };
    }
}

impl std::error::Error for ReadError {}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[derive(Debug)]
pub enum FileReadError {
    NotAFile,
    NotFound,
    CompressionError,
    Other(String),
}

impl std::fmt::Display for FileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for FileReadError {}

#[derive(Debug)]
pub enum EntryError {
    NothingFound,
    PathError,
    Other(String),
}

impl std::fmt::Display for EntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for EntryError {}

#[derive(Debug)]
pub enum ListingError {
    IsFile,
    Other(String),
}

impl std::fmt::Display for ListingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ListingError {}
