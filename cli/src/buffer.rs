use tokio::io::AsyncBufReadExt;

pub struct AsyncLineReader<R: tokio::io::AsyncRead> {
    reader: tokio::io::BufReader<R>,
    buffer: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Line {
    Ok(String),
    Empty,
    NoLF,
}

impl<R: tokio::io::AsyncRead + std::marker::Unpin> AsyncLineReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: tokio::io::BufReader::new(reader),
            buffer: String::new(),
        }
    }

    pub async fn read_line(&mut self) -> std::io::Result<Line> {
        Ok(if self.reader.read_line(&mut self.buffer).await? == 0 {
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

impl<R: tokio::io::AsyncRead> std::ops::Deref for AsyncLineReader<R> {
    type Target = tokio::io::BufReader<R>;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl<R: tokio::io::AsyncRead> std::ops::DerefMut for AsyncLineReader<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

impl<R: tokio::io::AsyncRead + std::marker::Unpin> std::convert::From<R> for AsyncLineReader<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod line_buf_reader_tests {
    use super::*;

    #[tokio::test]
    async fn empty() {
        let mut r = AsyncLineReader::new(std::io::Cursor::new("\n"));
        assert_eq!(Line::Ok(String::from("\n")), r.read_line().await.unwrap());
        assert_eq!(Line::Empty, r.read_line().await.unwrap());
        assert_eq!(Line::Empty, r.read_line().await.unwrap());
    }

    #[tokio::test]
    async fn no_lf() {
        let mut r = AsyncLineReader::new(std::io::Cursor::new(b"abc".to_vec()));
        assert_eq!(Line::NoLF, r.read_line().await.unwrap());
        r.get_mut().get_mut().extend(b"def\n");
        assert_eq!(Line::Ok(String::from("abcdef\n")), r.read_line().await.unwrap());
    }

    #[tokio::test]
    async fn preserve_buf_capacity() {
        let mut r = AsyncLineReader::new(std::io::Cursor::new(b"hello\n".to_vec()));
        _ = r.read_line().await;
        assert_eq!(0, r.buffer.len());
        assert!(6 <= r.buffer.capacity());
    }
}
