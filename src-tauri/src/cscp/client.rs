#![allow(non_snake_case, non_camel_case_types, unused_variables, unused_imports, dead_code)]
use std::{io::{Cursor, Error}, sync::Arc};
use bytes::{BytesMut, Buf, BufMut};
use futures_util::lock::Mutex;
use slab::Slab;
use tauri::Manager;
use tokio::{net::{TcpStream, ToSocketAddrs, tcp::{OwnedReadHalf, OwnedWriteHalf}}, sync::{mpsc, oneshot}};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use common::{Fader, DB, AudioType, AudioWidth, DeskInfo};

use crate::cscp::connection::Connection;

use super::{frame::{Frame, Message, FrameAck, FrameError, FrameMessage}, requests::Request};

pub type DeskInfoStorage = Arc<Mutex<Slab<DeskInfo>>>;
pub type FadersStorage = Arc<Mutex<Slab<Fader>>>;

pub struct CSCPClient;

impl CSCPClient {
  pub async fn connect<T: ToSocketAddrs>(addr: T, mut input_rx: mpsc::Receiver<Request>, fader_event_tx: mpsc::Sender<Fader>) -> Result<CSCPClient, Error> {
    let (to_mcs_tx, mut to_mcs_rx): (mpsc::Sender<Frame>, mpsc::Receiver<Frame>) = mpsc::channel(32);
    let (from_mcs_tx, mut from_mcs_rx): (mpsc::Sender<Frame>, mpsc::Receiver<Frame>) = mpsc::channel(32);
    let socket = TcpStream::connect(addr).await?;
    let connection = Connection::new(socket);
    let (mut read, mut write) = connection.split();
    println!("Connected");

    let faders_storage = FadersStorage::default();
    let inbound_faders_storage = faders_storage.clone();
    let desk_info_storage = DeskInfoStorage::default();
    let inbound_desk_info_storage = desk_info_storage.clone();

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
          println!("Reading CSCP frame");
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

    let inbound_listener = tokio::spawn(async move {
      loop {
        if let Some(req) = input_rx.recv().await {
          println!("Recv JS request {:?}", req);
          match req {
            Request::SET_FADER_LEVEL(msg) => {
              to_mcs_tx.send(Frame::set_fader_level(msg.index, msg.level)).await.unwrap();
            }
            Request::SET_FADER_CUT(msg) => {
              to_mcs_tx.send(Frame::set_fader_cut(msg.index, msg.isCut)).await.unwrap();
            }
            Request::SET_FADER_PFL(msg) => {
              to_mcs_tx.send(Frame::set_fader_pfl(msg.index, msg.isPfl)).await.unwrap();
            }
            Request::SET_MAIN_LEVEL(msg) => {
              to_mcs_tx.send(Frame::set_main_level(msg.index, msg.level)).await.unwrap();
            }
            Request::SET_MAIN_PFL(msg) => {
              to_mcs_tx.send(Frame::set_main_pfl(msg.index, msg.isPfl)).await.unwrap();
            }
            Request::GET_DB(sender) => {
              let mut db = DB::default();

              // Faders
              let faders = inbound_faders_storage.lock().await;
              let faders = faders.iter().map(|(_, book)| book).cloned().collect();
              db.faders = faders;

              // Desk info
              let deskInfo = inbound_desk_info_storage.lock().await;
              let deskInfos: Vec<&DeskInfo> = deskInfo.iter().map(|(_, book)| book).collect();
              if deskInfos.len() > 0 {
                db.deskInfo = deskInfo[0].clone();
              }

              sender.send(db).unwrap();
            }
          }
        }
      }
    });

    let outbound_listener = tokio::spawn(async move {
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
                  {
                    let mut fader = get_or_create_fader(&faders_storage, faderNum).await.unwrap();
                    fader.level = level;
                    update_fader(&faders_storage, fader.clone()).await.unwrap();

                    fader_event_tx.send(fader).await.unwrap();
                  }
                }
                0x01 => {
                  // Fader cut
                  // print("Fader", message[6], 'isCut', message[7] == 0)
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let isCut = buffer.get_u8() != 0;

                  // println!("FADER CUT :: faderNum={} isCut={}", faderNum, isCut);
                  {
                    let mut fader = get_or_create_fader(&faders_storage, faderNum).await.unwrap();
                    fader.isCut = isCut;
                    update_fader(&faders_storage, fader.clone()).await.unwrap();

                    fader_event_tx.send(fader).await.unwrap();
                  }
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
                  let isPfl = buffer.get_u8() != 0;
            
                  // println!("FADER PFL :: faderNum={} isPfl={}", faderNum, isPfl);
                  {
                    let mut fader = get_or_create_fader(&faders_storage, faderNum).await.unwrap();
                    fader.isPfl = isPfl;
                    update_fader(&faders_storage, fader.clone()).await.unwrap();

                    fader_event_tx.send(fader).await.unwrap();
                  }
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

                  {
                    update_desk_info(&desk_info_storage, DeskInfo { cscpVersion, numFaders, numMains, name }).await.unwrap();
                  }
                }
                0x0B => {
                  // Fader Label
                  buffer.set_position(5);
                  let faderNum = buffer.get_u16();
                  let label = String::from_utf8_lossy(&data.buffer[7..data.buffer.len() - 1]).to_string();
            
                  // println!("FADER LABEL :: faderNum={} label={}", faderNum, label);
                  {
                    let mut fader = get_or_create_fader(&faders_storage, faderNum).await.unwrap();
                    fader.label = label;
                    update_fader(&faders_storage, fader.clone()).await.unwrap();

                    fader_event_tx.send(fader).await.unwrap();
                  }
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
                  // let auxPage2 = buffer.get_u8();
                  // let auxPage3 = buffer.get_u8();

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
                  {
                    let mut fader = get_or_create_fader(&faders_storage, faderNum).await.unwrap();
                    fader.pathType = FromPrimitive::from_u8(audioType).unwrap_or(AudioType::U);
                    fader.format = FromPrimitive::from_u8(audioWidth).unwrap_or(AudioWidth::NP);
                    update_fader(&faders_storage, fader.clone()).await.unwrap();

                    fader_event_tx.send(fader).await.unwrap();
                  }
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
                  // let mainPage2 = buffer.get_u8();

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
    inbound_listener.await.unwrap();
    outbound_listener.await.unwrap();

    Ok(CSCPClient)
  }
}

async fn get_or_create_fader(faders_storage: &FadersStorage, index: u16) -> Result<Fader, ()> {
  let mut faders = faders_storage.lock().await;

  // if faders.contains(index as usize) {
  //   return Ok(faders.get(index as usize).cloned().unwrap());
  // }

  for (_, fader) in faders.iter() {
    if fader.index == index {
      println!("Return existing fader");
      return Ok(fader.clone());
    }
  }

  println!("Create new fader");
  let fader = Fader::new(index);
  let cloned_fader = fader.clone();
  let entry = faders.vacant_entry();
  entry.insert(fader);

  Ok(cloned_fader)
}

async fn update_fader(faders_storage: &FadersStorage, fader: Fader) -> Result<(), ()> {
  let mut faders = faders_storage.lock().await;

  let mut id: Option<usize> = None;

  for (entry_id, entry_fader) in faders.iter() {
    if entry_fader.index == fader.index {
      id = Some(entry_id);
    }
  }

  if id.is_some() {
    faders.remove(id.unwrap());
  }

  println!("UPDATED FADER :: {:?}", fader);
  
  let entry = faders.vacant_entry();
  entry.insert(fader);
  
  Ok(())
}

async fn update_desk_info(desk_info_storage: &DeskInfoStorage, desk_info: DeskInfo) -> Result<(), ()> {
  let mut info = desk_info_storage.lock().await;

  info.clear();

  let entry = info.vacant_entry();
  entry.insert(desk_info);
  Ok(())
}
