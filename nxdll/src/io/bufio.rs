use alloc::vec::Vec;
use nxdk_rs::embedded_io::ErrorType;
use nxdk_rs::embedded_io_async as embedded_io_async;

pub struct BufWriterAsync<W> {
    inner: W,
    buf: Vec<u8>,
    written: usize,
}

impl<W: embedded_io_async::Write> BufWriterAsync<W> {
    pub const DEFAULT_BUF_SIZE: usize = 8192;

    /// Creates a new `BufWriter` with a default buffer capacity
    pub fn new(inner: W) -> Self {
        Self::with_capacity(Self::DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufWriter` with the specified buffer capacity
    pub fn with_capacity(capacity: usize, inner: W) -> Self {
        Self {
            inner,
            buf: Vec::with_capacity(capacity),
            written: 0,
        }
    }

    /// Gets a reference to the underlying writer
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Gets a mutable reference to the underlying writer
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the buffer contents
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    /// Returns the number of bytes written through this buffer
    pub fn written(&self) -> usize {
        self.written
    }

    /// Flush the buffer if it is not empty
    pub async fn flush(&mut self) -> Result<(), W::Error> {
        if !self.buf.is_empty() {
            embedded_io_async::Write::write_all(&mut self.inner, &self.buf).await?;
            self.buf.clear();
        }
        Ok(())
    }

    pub async fn flush_inner(&mut self) -> Result<(), W::Error> {
        embedded_io_async::Write::flush(&mut self.inner).await?;
        self.inner.flush().await
    }

    /// Unwraps this `BufWriter`, returning the underlying writer
    pub async fn into_inner(mut self) -> Result<W, W::Error> {
        embedded_io_async::Write::flush(&mut self.inner).await?;
        Ok(self.inner)
    }
}

impl<W: embedded_io_async::Write> ErrorType for BufWriterAsync<W> { 
    type Error = W::Error;
}

impl<W: embedded_io_async::Write> embedded_io_async::Write for BufWriterAsync<W> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush().await?;
        }

        if buf.len() >= self.buf.capacity() {
            self.written += buf.len();
            embedded_io_async::Write::write_all(&mut self.inner, buf).await?;
            Ok(buf.len())
        } else {
            self.buf.extend_from_slice(buf);
            self.written += buf.len();
            Ok(buf.len())
        }
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        BufWriterAsync::flush(self).await
    }

    // FIXME: Implement max number of tries, or timeout
    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        let mut remaining = buf;
        while !remaining.is_empty() {
            let written = self.write(remaining).await?;
            remaining = &remaining[written..];
        }
        Ok(())
    }
}