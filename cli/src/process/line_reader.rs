use std::io::BufRead;

pub struct LineReader<R: std::io::Read> {
    reader: std::io::BufReader<R>,
    buf: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("buffer is empty")]
    Empty,
    #[error("reached to EOF before \\n")]
    NoNewLine,
}

impl<R: std::io::Read> LineReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: std::io::BufReader::new(reader),
            buf: String::new(),
        }
    }

    pub fn read_line(&mut self) -> Result<String, Error> {
        if self.reader.read_line(&mut self.buf)? == 0 {
            Err(Error::Empty)
        } else if !self.buf.ends_with('\n') {
            Err(Error::NoNewLine)
        } else {
            let s = self.buf.to_string();
            self.buf.clear();
            Ok(s)
        }
    }
}

impl<R: std::io::Read> std::ops::Deref for LineReader<R> {
    type Target = std::io::BufReader<R>;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl<R: std::io::Read> std::ops::DerefMut for LineReader<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_empty() {
        let mut r = LineReader::new(std::io::Cursor::new("\n"));
        assert_eq!(String::from("\n"), r.read_line().unwrap());
        assert!(matches!(r.read_line().unwrap_err(), Error::Empty));
        assert!(matches!(r.read_line().unwrap_err(), Error::Empty));
    }

    #[test]
    fn error_nonewline() {
        let mut r = LineReader::new(std::io::Cursor::new(b"abc".to_vec()));
        assert!(matches!(r.read_line().unwrap_err(), Error::NoNewLine));
        r.get_mut().get_mut().extend(b"def\n");
        assert_eq!(String::from("abcdef\n"), r.read_line().unwrap());
    }

    #[test]
    fn preserve_buf_capacity() {
        let mut r = LineReader::new(std::io::Cursor::new(b"hello\n".to_vec()));
        _ = r.read_line();
        assert_eq!(0, r.buf.len());
        assert!(6 <= r.buf.capacity());
    }
}
