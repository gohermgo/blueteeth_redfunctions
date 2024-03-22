use std::{borrow::BorrowMut, error::Error};

use btleplug::{
    api::{bleuuid::BleUuid, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter},
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
    pub fn start(mut self) -> anyhow::Result<PeripheralTerminal> {
        let (peripheral_tx, peripheral_rx) =
            tokio::sync::mpsc::channel(std::mem::size_of::<Peripheral>());
        let adapter = self.adapter.clone();
        tokio::task::spawn(async move {
            let _ = adapter.start_scan(ScanFilter::default()).await.unwrap();
            loop {
                match self.discovery_rx.recv().await {
                    Some(p) => match peripheral_tx.send(p).await {
                        Ok(()) => continue,
                        // We closed the receiver
                        _ => break,
                    },
                    None => continue,
                }
            }
        });
        Ok(PeripheralTerminal { peripheral_rx })
    }
}
// impl Drop for BluetoothScanner {
//     fn drop(&mut self) {
//         self.discovery_rx.close()
//     }
// }
pub struct PeripheralTerminal {
    peripheral_rx: tokio::sync::mpsc::Receiver<Peripheral>,
}
impl PeripheralTerminal {
    pub fn new() -> anyhow::Result<Self> {
        let bt_scanner = BluetoothScanner::new()?;
        bt_scanner.start()
    }
    pub fn filter_reception(mut self, query: DeviceQuery) -> anyhow::Result<tokio::sync::mpsc::UnboundedReceiver<Peripheral>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn(async move {
            loop {
                let (props, p)= match self.peripheral_rx.recv().await {
                    Some(p) => (p.properties().await.unwrap().unwrap(), p),
                    None => continue
                };
                match query {
                    DeviceQuery::ByName(ref name) if props.local_name.unwrap_or_default().eq(name) => tx.send(p).unwrap(),
                    DeviceQuery::ByUuid(uuid) if props.services.into_iter().find(|value| value.eq(&uuid)).is_some() => tx.send(p).unwrap(),
                    DeviceQuery::ByAddress(ref addr) if props.address.to_string().eq(addr) => tx.send(p).unwrap(),
                    _ => continue
                }
            }
        });
        Ok(rx)
    }
}
impl Drop for PeripheralTerminal {
    fn drop(&mut self) {
        self.peripheral_rx.close();
    }
}
pub struct PeripheralSearcher {
    terminal: PeripheralTerminal
}
impl PeripheralSearcher {
    pub fn new() -> anyhow::Result<Self> {
        let x = tokio::task::spawn()
    }
    pub fn search_by(self, query: DeviceQuery) -> anyhow::Result<Peripheral> {
        todo!()
    }
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
