use std::{io::{Cursor}};
use bytes::{BytesMut, Buf};
use tokio::{net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}}};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use super::frame::{Frame, Message, FrameAck, FrameError, FrameMessage};
pub struct Connection {
  stream: TcpStream,
  buffer: BytesMut,
}

pub struct ConnectionRead {
  stream: OwnedReadHalf,
  buffer: BytesMut,
}

impl ConnectionRead {
  // @TODO reuse these methods from Connection
  pub async fn read_frame(&mut self) -> Result<Option<Frame>, String> {
    loop {
      // println!("Before read frame");
      if let Some(frame) = self.parse_frame()? {
        // println!("Got frame");
        return Ok(Some(frame));
      }

      // println!("No frame yet {:?}", self.buffer);

      match self.stream.read_buf(&mut self.buffer).await {
        Ok(0) => {
          println!("buffer empty, stop");
          panic!("Dead stream mate");
        }
        Ok(n) => {
          println!("Read {} bytes from stream", n);
          return Ok(None);
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
          // println!("Could block?");
          return Ok(None);
        }
        Err(e) => {
            return Err(e.to_string());
        }
      }
    }
  }

  fn parse_frame(&mut self) -> Result<Option<Frame>, String> {
    let mut buf = Cursor::new(&self.buffer[..]);

    match Frame::check(&mut buf) {
      Ok(_) => {
        buf.set_position(0);

        match buf.get_u8() {
          0x04 => {
            // println!("Read ACK");
            self.buffer.advance(1);
            return Ok(Some(Frame::new(Message::ACK(FrameAck{}))));
          }
          0x05 => {
            // println!("Read ERR");
            let frame = Frame::new(Message::ERR(FrameError{ error: buf.get_u8() }));

            self.buffer.advance(2);

            return Ok(Some(frame));
          }
          _ => {
            // println!("Read MSG");
            let len = (buf.get_u8() + 4) as usize;
            buf.set_position(0);

            let next_buffer = get_frame(&mut buf, len).to_vec();
            let frame = Frame::new(Message::MSG(FrameMessage{ buffer: next_buffer }));
    
            self.buffer.advance(len);
    
            return Ok(Some(frame));
          }
        }
      }
      Err(Incomplete) => Ok(None),
      // Err(e) => Err(e.to_string()),
    }
  }
}

pub struct ConnectionWrite {
  stream: OwnedWriteHalf,
}

impl ConnectionWrite {
  pub async fn write_frame(&mut self, frame: Frame) -> Result<(), String> {
    match frame.msg {
      Message::MSG(msg) => {
        self.stream.write_all(&msg.buffer).await.expect("Could not write to stream");
        self.stream.flush().await.unwrap();
      }
      _ => {}
    }

    Ok(())
  }
}

impl Connection {
  pub fn new(stream: TcpStream) -> Connection {
    Connection {
      stream,
      buffer: BytesMut::with_capacity(4096),
    }
  }

  pub fn split(self) -> (ConnectionRead, ConnectionWrite) {
    let (rx, tx)  = self.stream.into_split();
    (ConnectionRead { stream: rx, buffer: self.buffer }, ConnectionWrite { stream: tx })
  }
}

fn get_frame<'a>(src: &mut Cursor<&'a [u8]>, end: usize) -> &'a [u8] {
  let start = 0;
  return &src.get_ref()[start..end];
}
