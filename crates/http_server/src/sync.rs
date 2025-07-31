use tokio::{io::AsyncRead, sync::mpsc};

pub struct ChannelReader {
    rx: mpsc::Receiver<u8>,
}

impl ChannelReader {
    pub fn new(rx: mpsc::Receiver<u8>) -> Self {
        Self { rx }
    }
}

impl AsyncRead for ChannelReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::task::Poll;
        let mut read_bytes = 0;
        while buf.remaining() > 0 {
            match self.rx.poll_recv(cx) {
                Poll::Ready(Some(byte)) => {
                    buf.put_slice(&[byte]);
                    read_bytes += 1;
                }
                Poll::Ready(None) => break,
                Poll::Pending => {
                    return if read_bytes == 0 {
                        Poll::Pending
                    } else {
                        Poll::Ready(Ok(()))
                    };
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{io::AsyncReadExt, time::sleep};

    use super::*;

    #[tokio::test]
    async fn test_channel_reader() {
        const LINE: &'static [u8] = b"GET / HTTP/1.1\r\nHost: test\r\n\r\n";
        fn setup_reader() -> ChannelReader {
            let (tx, rx) = mpsc::channel::<u8>(LINE.len());

            tokio::spawn(async move {
                sleep(Duration::from_millis(10)).await;
                for ch in LINE.iter() {
                    tx.send(*ch).await.unwrap();
                    sleep(Duration::from_millis(10)).await;
                }
            });
            ChannelReader::new(rx)
        }

        {
            // Test with read_exact
            let mut reader = setup_reader();
            let mut buf = [0u8; LINE.len()];

            let n = reader.read_exact(&mut buf).await.unwrap();
            assert_ne!(n, 0);
            assert_eq!(&buf, LINE);
        }

        {
            // Test with read
            let mut reader = setup_reader();
            let mut buf = [0u8; LINE.len()];
            let mut bytes_read = 0;

            while bytes_read < LINE.len() {
                let n = reader.read(&mut buf[bytes_read..]).await.unwrap();
                bytes_read += n;
                assert_ne!(n, 0);
            }
            assert_eq!(&buf[..], LINE);
        }
    }
}
