use common::DB;
use tauri::{async_runtime::Mutex};
use tokio::sync::{mpsc, oneshot};

use crate::cscp::requests::{SetFaderLevel, SetFaderCut, SetFaderPfl};

use super::requests::Request;

pub struct AsyncProcInputTx {
  pub inner: Mutex<mpsc::Sender<Request>>,
}

#[tauri::command]
pub async fn setFaderLevel(
    index: u16,
    level: u16,
    state: tauri::State<'_, AsyncProcInputTx>,
) -> Result<(), String> {
    println!("setFaderLevel faderNum={} level={}", index, level);
    // info!(?message, "js2rs");
    let async_proc_input_tx = state.inner.lock().await;
    async_proc_input_tx
        .send(Request::SET_FADER_LEVEL(SetFaderLevel { index, level }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn setFaderCut(
    index: u16,
    isCut: bool,
    state: tauri::State<'_, AsyncProcInputTx>,
) -> Result<(), String> {
    println!("setFaderCut faderNum={} isCut={}", index, isCut);
    // info!(?message, "js2rs");
    let async_proc_input_tx = state.inner.lock().await;
    async_proc_input_tx
        .send(Request::SET_FADER_CUT(SetFaderCut { index, isCut }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn setFaderPfl(
    index: u16,
    isPfl: bool,
    state: tauri::State<'_, AsyncProcInputTx>,
) -> Result<(), String> {
    println!("setFaderPfl faderNum={} isPfl={}", index, isPfl);
    // info!(?message, "js2rs");
    let async_proc_input_tx = state.inner.lock().await;
    async_proc_input_tx
        .send(Request::SET_FADER_PFL(SetFaderPfl { index, isPfl }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn getDatabase(
    state: tauri::State<'_, AsyncProcInputTx>,
) -> Result<DB, String> {
    println!("Send DB");
    // info!(?message, "js2rs");
    let (single_tx, single_rx) = oneshot::channel();
    let async_proc_input_tx = state.inner.lock().await;

    async_proc_input_tx
        .send(Request::GET_DB(single_tx))
        .await
        .unwrap();

    let res = single_rx.await.unwrap();

    println!("DB {:?}", res);

    Ok(res)
}
