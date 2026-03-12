use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::{fs, io};

const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;

pub struct Source {
    pub path_buf: Option<PathBuf>,
    pub content: String, // TODO(@pxc1984): add memory mapping for large files
}

#[derive(Debug)]
pub enum ErrorKind {
    Io(io::Error),
    FileTooBig(PathBuf),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Io(err) => err.fmt(f),
            ErrorKind::FileTooBig(path) => {
                write!(f, "file {} is too big (>{MAX_FILE_SIZE}", path.display())
            }
        }
    }
}

impl Source {
    pub fn open(path_buf: Option<PathBuf>) -> Result<Self, ErrorKind> {
        let mut source: Box<dyn BufRead> = match &path_buf {
            None => Box::new(BufReader::new(io::stdin())),
            Some(path) => {
                let metadata = fs::metadata(path.clone()).map_err(ErrorKind::Io)?;
                if metadata.len() > MAX_FILE_SIZE {
                    return Err(ErrorKind::FileTooBig(path.clone()));
                }

                Box::new(BufReader::new(File::open(path).map_err(ErrorKind::Io)?))
            }
        };
        let mut content = String::new();
        source.read_to_string(&mut content).map_err(ErrorKind::Io)?;

        Ok(Source { path_buf, content })
    }
}
