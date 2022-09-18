#![allow(non_snake_case, non_camel_case_types)]
use std::{io::{Cursor, Error}, sync::Arc};
use bytes::{BytesMut, Buf, BufMut};
use futures_util::lock::Mutex;
use slab::Slab;
use tokio::{net::{TcpStream, ToSocketAddrs, tcp::{OwnedReadHalf, OwnedWriteHalf}}, sync::mpsc};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

enum AudioType {
  U,
  CH,
  GP,
  VCA_MASTER,
  VCA_MASTER_CH,
  VCA_MASTER_GP,
  MN,
  VCA_MASTER_MN,
  TK,
  VCA_MASTER_TK,
  AUX,
  VCA_MASTER_AUX,
}

enum AudioWidth {
  NP,
  M,
  ST,
  UNUSED1,
  UNUSED2,
  UNUSED3,
  SU,
}

pub struct Fader {
  pub index: u16,
  pub label: String,
  pub level: u16,
  pub isCut: bool,
  pub isPfl: bool,
}

impl Fader {
  pub fn new(index: u16) -> Fader {
    Fader { index, label: String::from(""), level: 0, isCut: false, isPfl: false }
  }
}

pub struct Main {
  pub index: u16,
  pub label: String,
  pub level: u16,
  pub isPfl: bool,
}

impl Main {
  pub fn new(index: u16) -> Main {
    Main { index, label: String::from(""), level: 0, isPfl: false }
  }
}

pub struct DeskInfo {
  pub cscpVersion: u16,
  pub numFaders: u16,
  pub numMains: u16,
  pub name: String,
}

pub type FadersStorage = Arc<Mutex<Slab<Fader>>>;
pub type MainsStorage = Arc<Mutex<Slab<Main>>>;

#[derive(Debug, Clone)]
pub struct FrameAck {}


#[derive(Debug, Clone)]pub struct FrameError {
  pub error: u8,
}

#[derive(Debug, Clone)]
pub struct FrameMessage {
  pub buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum Message {
  ACK(FrameAck),
  ERR(FrameError),
  MSG(FrameMessage),
}

#[derive(Debug, Clone)]
pub struct Frame {
  pub msg: Message,
}

impl Frame {
  pub fn new(msg: Message) -> Frame {
    Frame { msg }
  }

  pub fn set_fader_level(fader_number: u16, value: u16) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    let mut value_buffer = BytesMut::with_capacity(2);
    value_buffer.put_u16(value);
    Frame::send(vec![0x80, 0x00], fader_number_buffer.to_vec(), value_buffer.to_vec())
  }

  pub fn set_fader_cut(fader_number: u16, is_on: bool) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    let mut value_buffer = BytesMut::with_capacity(1);
    value_buffer.put_u8(is_on as u8);
    Frame::send(vec![0x80, 0x01], fader_number_buffer.to_vec(), value_buffer.to_vec())
  }

  pub fn set_fader_pfl(fader_number: u16, is_on: bool) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    let mut value_buffer = BytesMut::with_capacity(1);
    value_buffer.put_u8(is_on as u8);
    Frame::send(vec![0x80, 0x05], fader_number_buffer.to_vec(), value_buffer.to_vec())
  }

  pub fn set_main_level(main_number: u16, value: u16) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(main_number);
    let mut value_buffer = BytesMut::with_capacity(2);
    value_buffer.put_u16(value);
    Frame::send(vec![0x80, 0x02], fader_number_buffer.to_vec(), value_buffer.to_vec())
  }

  pub fn set_main_pfl(main_number: u16, is_on: bool) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(main_number);
    let mut value_buffer = BytesMut::with_capacity(1);
    value_buffer.put_u8(is_on as u8);
    Frame::send(vec![0x80, 0x0C], fader_number_buffer.to_vec(), value_buffer.to_vec())
  }

  pub fn get_console_name() -> Frame {
    Frame::send(vec![0x00, 0x07], vec![], vec![])
  }

  pub fn get_console_info() -> Frame {
    Frame::send(vec![0x00, 0x08], vec![], vec![])
  }

  pub fn get_fader_level(fader_number: u16) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    Frame::send(vec![0x00, 0x00], fader_number_buffer.to_vec(), vec![])
  }

  pub fn get_fader_cut(fader_number: u16) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    Frame::send(vec![0x00, 0x01], fader_number_buffer.to_vec(), vec![])
  }

  pub fn get_fader_pfl(fader_number: u16) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    Frame::send(vec![0x00, 0x05], fader_number_buffer.to_vec(), vec![])
  }

  pub fn get_fader_label(fader_number: u16) -> Frame {
    let mut fader_number_buffer = BytesMut::with_capacity(2);
    fader_number_buffer.put_u16(fader_number);
    Frame::send(vec![0x00, 0x0B], fader_number_buffer.to_vec(), vec![])
  }

  pub fn send(cmd: Vec<u8>, data: Vec<u8>, value: Vec<u8>) -> Frame {
    let byte_count = (cmd.len() + data.len() + value.len()) as u8;
    let cmd_sum = sum(&cmd);
    let data_sum = sum(&data);
    let value_sum = sum(&value);
    let byte_sum = cmd_sum + data_sum + value_sum;
    let mut header_buffer = BytesMut::with_capacity(3);
    header_buffer.put_u8(0xF1);
    header_buffer.put_u8(byte_count);
    header_buffer.put_u8(0x00);
    let mut sum_buffer = BytesMut::with_capacity(1);
    sum_buffer.put_u8(byte_sum.wrapping_neg() as u8);

    let outgoing = [header_buffer.to_vec(), cmd, data, value, sum_buffer.to_vec()].concat();
    Frame::new(Message::MSG(FrameMessage{ buffer: outgoing.to_vec() }))
  }

  pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), String> {
    let remaining = src.remaining();

    if remaining < 1 { return Err("NoFrame".to_string()); }

    match src.get_u8() {
      0x04 => {
        return Ok(());
      }
      0x05 => {
        if remaining >= 2{
          return Ok(());
        }
        return Err("NoFrame".to_string());
      }
      _ => {
        if remaining < 4 { return Err("NoFrame".to_string()); }
        src.set_position(1);
        let len = (src.get_u8() + 4) as usize;
        if len <= remaining {
          return Ok(());
        }
    
        return Err("AlsoNotAFrame".to_string());
      }
    }
  }
}

struct Connection {
  stream: TcpStream,
  buffer: BytesMut,
}

struct ConnectionRead {
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
      Err(e) => Err(e.to_string()),
    }
  }
}

struct ConnectionWrite {
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

fn sum(list: &Vec<u8>) -> u16 {
  list.iter().map(|&i| i as u16).sum()
}

struct CSCPClient;

impl CSCPClient {
  pub async fn connect<T: ToSocketAddrs>(addr: T) -> Result<CSCPClient, Error> {
    let (to_mcs_tx, mut to_mcs_rx): (mpsc::Sender<Frame>, mpsc::Receiver<Frame>) = mpsc::channel(32);
    let (from_mcs_tx, mut from_mcs_rx): (mpsc::Sender<Frame>, mpsc::Receiver<Frame>) = mpsc::channel(32);
    let socket = TcpStream::connect(addr).await?;
    let connection = Connection::new(socket);
    let (mut read, mut write) = connection.split();
    println!("Connected");

    // to_mcs_tx.send(Frame::set_fader_level(3, 744)).await.unwrap();
    to_mcs_tx.send(Frame::get_console_info()).await.unwrap();
    // to_mcs_tx.send(Frame::get_fader_pfl(3)).await.unwrap();
    // to_mcs_tx.send(Frame::set_fader_pfl(3, false)).await.unwrap();
    // to_mcs_tx.send(Frame::get_fader_label(3)).await.unwrap();

    // self.set_fader_level(0x01, 1024);

    let read_manager =  tokio::spawn(async move {
      loop {
        // println!("Before read frame");
        if let Some(frame) = read.read_frame().await.unwrap() {
          // println!("Reading CSCP frame");
          from_mcs_tx.send(frame).await.unwrap();
          // println!("Posted CSCP frame");
        }
      }
    });

    let write_manager = tokio::spawn(async move {
      loop {
        // println!("Before try recv");
        if let Some(frame) = to_mcs_rx.recv().await {
          println!("Forward CSCP MSG {:?}", frame);
          write.write_frame(frame).await.unwrap();
        }
      }
    });

    let listener = tokio::spawn(async move {
      println!("Start listener");
      loop {
        if let Some(frame) = from_mcs_rx.recv().await {
          // println!("CSCP MSG {:?}", msg);
          match frame.msg {
            Message::ACK(_) => {
              println!("ACK");
            }
            Message::ERR(error) => {
              println!("Error {}", error.error);
            }
            Message::MSG(data) => {
              let mut buffer = Cursor::new(&data.buffer[..]);
              buffer.set_position(4);
              match buffer.get_u8() {
                0x00 => {
                  // Fader level
                  // print("Fader", message[6], 'level', (message[7] << 8) | message[8])
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let level = buffer.get_u16();
            
                  // println!("FADER LEVEL :: faderNum={} level={}", faderNum, level);
                }
                0x01 => {
                  // Fader cut
                  // print("Fader", message[6], 'isCut', message[7] == 0)
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let isCut = buffer.get_u8() == 0;
            
                  // println!("FADER CUT :: faderNum={} isCut={}", faderNum, isCut);
                }
                0x02 => {
                  // Main level
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let level = buffer.get_u16();
            
                  // println!("MAIN LEVEL :: MN={} level={}", faderNum, level);
                }
                0x05 => {
                  // Fader PFL
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let isPfl = buffer.get_u8() == 0;
            
                  // println!("FADER PFL :: faderNum={} isPfl={}", faderNum, isPfl);
                }
                0x07 => {
                  // Desk name
                  // print("Desk name", message[5:-1].decode('utf-8'))
                  let name = String::from_utf8_lossy(&data.buffer[5..data.buffer.len() - 1]).to_string();
            
                  println!("DESK NAME :: name={}", name);
                }
                0x08 => {
                  // Desk Info
                  // print("Desk info version=", message[6], "faders=", message[8], "mains=", message[10], "name=", message[17:-1].decode('utf-8'))
                  buffer.set_position(5);
                  let cscpVersion = buffer.get_u16();
                  let numFaders = buffer.get_u16();
                  let numMains = buffer.get_u16();
                  let name = String::from_utf8_lossy(&data.buffer[17..data.buffer.len() - 1]).to_string();
            
                  println!("DESK INFO :: cscpVersion={}, numFaders={}, numMains={}, name={}", cscpVersion, numFaders, numMains, name);
                }
                0x0B => {
                  // Fader Label
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let label = String::from_utf8_lossy(&data.buffer[7..data.buffer.len() - 1]).to_string();
            
                  // println!("FADER LABEL :: faderNum={} label={}", faderNum, label);
                }
                0x0C => {
                  // Main PFL
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let isPfl = buffer.get_u8() == 0;
            
                  // println!("MAIN PFL :: MN={} isPfl={}", faderNum, isPfl);
                }
                0x0D => {
                  // Main Label
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let label = String::from_utf8_lossy(&data.buffer[7..data.buffer.len() - 1]).to_string();
            
                  // println!("MAIN LABEL :: MN={} label={}", faderNum, label);
                }
                0x10 => {
                  // Aux availability
                  buffer.set_position(5);
                  let auxPage1 = buffer.get_u8();
                  let auxPage2 = buffer.get_u8();
                  let auxPage3 = buffer.get_u8();

                  let aux1 = auxPage1 >> 0 & 1 != 0;
                  let aux2 = auxPage1 >> 1 & 1 != 0;
                  let aux3 = auxPage1 >> 2 & 1 != 0;
                  let aux4 = auxPage1 >> 3 & 1 != 0;
                  let aux5 = auxPage1 >> 4 & 1 != 0;
                  let aux6 = auxPage1 >> 5 & 1 != 0;
                  let aux7 = auxPage1 >> 6 & 1 != 0;
                  let aux8 = auxPage1 >> 7 & 1 != 0;

                  println!("AUXES :: auxes={:?}", vec![aux1, aux2, aux3, aux4, aux5, aux6, aux7, aux8]);
                }
                0x11 => {
                  // Fader format
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let audioType = buffer.get_u8();
                  let audioWidth = buffer.get_u8();

                  // println!("FADER FORMAT :: faderNum={} audioType={} audioWidth={}", faderNum, audioType, audioWidth);
                }
                0x13 => {
                  // Aux level
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let level = buffer.get_u16();
            
                  println!("AUX LEVEL :: AUX={} level={}", faderNum, level);
                }
                0x14 => {
                  // Main availability
                  buffer.set_position(5);
                  let mainPage1 = buffer.get_u8();
                  let mainPage2 = buffer.get_u8();

                  let main1 = mainPage1 >> 0 & 1 != 0;
                  let main2 = mainPage1 >> 1 & 1 != 0;
                  let main3 = mainPage1 >> 2 & 1 != 0;

                  println!("MAINS :: mains={:?}", vec![main1, main2, main3]);
                }
                0x16 => {
                  // Input?
                  buffer.set_position(5);
                  let fader1to4 = buffer.get_u8();
                  // bit 0 = fader 0 L > B
                  // bit 1 = fader 0 R > B
                  // but 2 = fader 1 L > B
                  // etc..

                  println!("INPUT :: buffer={:?}", data);
                }
                _ => {}
              }
            }
          }
        }
      }
    });

    read_manager.await.unwrap();
    write_manager.await.unwrap();
    listener.await.unwrap();

    Ok(CSCPClient)
  }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let mcs = tokio::spawn(async move {
    let client = CSCPClient::connect("172.16.255.5:49556").await.unwrap();
    println!("Client finished");
  });

  mcs.await.unwrap();

  Ok(())
}
