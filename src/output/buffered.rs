use std::io::Write;
use std::sync::{Arc, Mutex};

/// A Write decorator that buffers lines and then writes the output to the inner Write
pub struct LineWriteDecorator<'a> {
    inner: &'a mut (dyn Write + Send),
    buffer: Vec<u8>,
    write_mutex: Arc<Mutex<()>>,
}

impl<'a> LineWriteDecorator<'a> {
    pub fn new(inner: &'a mut (dyn Write + Send), write_mutex: Arc<Mutex<()>>) -> Self {
        Self {
            inner,
            buffer: Vec::with_capacity(256),
            write_mutex,
        }
    }
}

impl Write for LineWriteDecorator<'_> {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        for &i in input {
            self.buffer.push(i);
            if i == b'\n' {
                self.flush()?;
            }
        }
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // Lock mutex to ensure not writing lines to stdout and stderr at the same time. If the
        // another thread panicked we proceed anyway.
        let _lock = self.write_mutex.lock().ok();
        self.inner.write_all(self.buffer.as_slice())?;
        self.buffer.clear();
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorator_does_not_write_to_inner_without_newline() {
        let mut inner = Vec::<u8>::new();
        let mutex = Arc::new(Mutex::new(()));
        let mut decorator = LineWriteDecorator::new(&mut inner, mutex);
        assert_eq!(5, decorator.write(b"Hello").unwrap());
        assert!(inner.is_empty());
    }

    #[test]
    fn decorator_writes_to_inner_with_newline() {
        let mut inner = Vec::<u8>::new();
        let mutex = Arc::new(Mutex::new(()));
        let mut decorator = LineWriteDecorator::new(&mut inner, mutex);
        assert_eq!(6, decorator.write(b"Hello\n").unwrap());
        assert_eq!(b"Hello\n", inner.as_slice());
    }

    #[test]
    fn decorator_writes_to_inner_at_flush() {
        let mut inner = Vec::<u8>::new();
        let mutex = Arc::new(Mutex::new(()));
        let mut decorator = LineWriteDecorator::new(&mut inner, mutex);
        assert_eq!(5, decorator.write(b"Hello").unwrap());
        decorator.flush().unwrap();
        assert_eq!(b"Hello", inner.as_slice());
    }
}
