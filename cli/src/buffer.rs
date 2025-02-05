use std::io::BufRead;

pub struct LineBufReader<R: std::io::Read> {
    reader: std::io::BufReader<R>,
    buffer: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Line {
    Ok(String),
    Empty,
    NoLF,
}

impl<R: std::io::Read> LineBufReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: std::io::BufReader::new(reader),
            buffer: String::new(),
        }
    }

    pub fn read_line(&mut self) -> std::io::Result<Line> {
        Ok(if self.reader.read_line(&mut self.buffer)? == 0 {
            Line::Empty
        } else if !self.buffer.ends_with('\n') {
            Line::NoLF
        } else {
            let s = self.buffer.to_string();
            self.buffer.clear();
            Line::Ok(s)
        })
    }
}

impl<R: std::io::Read> std::ops::Deref for LineBufReader<R> {
    type Target = std::io::BufReader<R>;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl<R: std::io::Read> std::ops::DerefMut for LineBufReader<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

impl<R: std::io::Read> std::convert::From<R> for LineBufReader<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod line_buf_reader_tests {
    use super::*;

    #[test]
    fn empty() {
        let mut r = LineBufReader::new(std::io::Cursor::new("\n"));
        assert_eq!(Line::Ok(String::from("\n")), r.read_line().unwrap());
        assert_eq!(Line::Empty, r.read_line().unwrap());
        assert_eq!(Line::Empty, r.read_line().unwrap());
    }

    #[test]
    fn no_lf() {
        let mut r = LineBufReader::new(std::io::Cursor::new(b"abc".to_vec()));
        assert_eq!(Line::NoLF, r.read_line().unwrap());
        r.get_mut().get_mut().extend(b"def\n");
        assert_eq!(Line::Ok(String::from("abcdef\n")), r.read_line().unwrap());
    }

    #[test]
    fn preserve_buf_capacity() {
        let mut r = LineBufReader::new(std::io::Cursor::new(b"hello\n".to_vec()));
        _ = r.read_line();
        assert_eq!(0, r.buffer.len());
        assert!(6 <= r.buffer.capacity());
    }
}
