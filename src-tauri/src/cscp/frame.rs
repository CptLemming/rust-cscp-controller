use std::io::Cursor;

use bytes::{BytesMut, Buf, BufMut};

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

fn sum(list: &Vec<u8>) -> u16 {
  list.iter().map(|&i| i as u16).sum()
}
