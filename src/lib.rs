use std::{borrow::BorrowMut, error::Error};

use btleplug::{
    api::{bleuuid::BleUuid, Central, CentralEvent, Manager as _, ScanFilter},
    platform::{Adapter, Manager, Peripheral},
};
use futures::stream::StreamExt;
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::oneshot::error::TryRecvError;

use uuid::Uuid;

async fn disconnect_handler() -> anyhow::Result<()> {
    todo!()
}
pub enum DeviceQuery {
    ByName(String),
    ByUuid(Uuid),
    ByAddress(String),
}
pub struct BluetoothEventWatcher;
pub struct ManagedAdapter {
    manager: Manager,
    adapter: Adapter,
}
async fn get_central_adapter() -> Result<Adapter, btleplug::Error> {
    let m = Manager::new().await?;
    m.adapters()
        .await?
        .into_iter()
        .nth(0)
        .ok_or(btleplug::Error::RuntimeError(
            "Failed to get central adapter".into(),
        ))
}
async fn get_managed_peripheral(
) -> Result<(Manager, <Adapter as Central>::Peripheral), btleplug::Error> {
    // let (manager, adapter) = get_adapter().await?;

    todo!()
}
// impl ManagedAdapter {
//     pub fn new() -> anyhow::Result<Self> {
//         // Get manager
//         let (tx, rx) = std::sync::mpsc::channel();
//         tokio::task::spawn(async move {
//             let _ = tx.send(get_central_adapter().await);
//         });

//         Ok(ManagedAdapter {
//             manager,
//             adapter: adapters,
//         })
//     }
//     pub fn start_scan(&self) -> anyhow::Result<BluetoothScanner> {
//         tokio::task::spawn(async move {
//             for adapter in self.adapter {
//                 adapter
//                     .start_scan(ScanFilter::default())
//                     .await
//                     .unwrap_or_default()
//             }
//         });
//         todo!()
//     }
//     pub fn peripherals(&self) -> anyhow::Result<<Adapter as Central>::Peripheral> {
//         todo!()
//     }
// }
pub struct BluetoothScanner {
    adapter: std::sync::Arc<Adapter>,
    discovery_rx: tokio::sync::mpsc::Receiver<Peripheral>,
}
impl BluetoothScanner {
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        tokio::task::spawn(async move {
            let _ = tx.send(get_central_adapter().await);
        });
        let adapter_arc = Arc::new(rx.recv()??);
        let adapter = adapter_arc.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        tokio::task::spawn(async move {
            let _ = tx.send(adapter.events().await);
        });
        let mut event_stream = rx.recv()??;
        let adapter = adapter_arc.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(std::mem::size_of::<Peripheral>());
        tokio::task::spawn(async move {
            while let Some(event) = event_stream.next().await {
                match event {
                    CentralEvent::DeviceDiscovered(id) => {
                        let peripheral = adapter.peripheral(&id).await.unwrap();
                        tx.send(peripheral).await.unwrap_or_default()
                    }
                    _ => continue,
                }
            }
        });
        Ok(Self {
            adapter: adapter_arc,
            discovery_rx: rx,
        })
    }
    pub fn start(mut self) -> anyhow::Result<PeripheralHandler> {
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();
        let (peripheral_tx, peripheral_rx) =
            tokio::sync::mpsc::channel(std::mem::size_of::<Peripheral>());
        let adapter = self.adapter.clone();
        tokio::task::spawn(async move {
            let _ = adapter.start_scan(ScanFilter::default()).await.unwrap();
            loop {
                match (self.discovery_rx.recv().await, stop_rx.try_recv()) {
                    (Some(p), Err(TryRecvError::Closed)) | (Some(p), Ok(())) => {
                        peripheral_tx.send(p).await.unwrap();
                        break;
                    }
                    (Some(p), _) => match peripheral_tx.send(p).await {
                        Ok(()) => continue,
                        // We closed the receiver
                        _ => break,
                    },
                    (None, Err(TryRecvError::Closed)) | (_, Ok(())) => break,
                    (None, _) => continue,
                }
            }
        });
        Ok(PeripheralHandler {
            stop_tx,
            peripheral_rx,
        })
    }
}
impl Drop for BluetoothScanner {
    fn drop(&mut self) {
        self.discovery_rx.close()
    }
}
pub struct PeripheralHandler {
    stop_tx: tokio::sync::oneshot::Sender<()>,
    peripheral_rx: tokio::sync::mpsc::Receiver<Peripheral>,
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
