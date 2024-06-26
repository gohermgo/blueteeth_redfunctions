//! This is really cool no?
use std::time::{Duration, Instant};
use tokio::sync::mpsc::error::SendError;
use windows::{
    core::{IInspectable, GUID, HSTRING},
    Devices::Bluetooth::{
        Advertisement::{
            BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
        },
        BluetoothConnectionStatus, BluetoothLEDevice,
        GenericAttributeProfile::GattDeviceService,
    },
    Foundation::{EventRegistrationToken, TypedEventHandler},
};

pub struct DeviceAddress {
    address: [u8; 6],
}

impl From<DeviceAddress> for u64 {
    fn from(addr: DeviceAddress) -> Self {
        let mut slice = [0; 8];
        slice[2..].copy_from_slice(&addr.address);
        u64::from_be_bytes(slice)
    }
}
pub struct DeviceSniffer {}
impl DeviceSniffer {
    const TARGET: GUID = GUID::zeroed();
    pub fn sniff(
        guid_tx: tokio::sync::mpsc::UnboundedSender<GUID>,
        token_tx: tokio::sync::mpsc::UnboundedSender<EventRegistrationToken>,
    ) -> windows::core::Result<()> {
        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        let token_handler = TypedEventHandler::new(
            move |_, opt_args: &Option<BluetoothLEAdvertisementReceivedEventArgs>| {
                if let Some(args) = opt_args {
                    let advertisement = args.Advertisement()?;
                    let service_uuids = advertisement.ServiceUuids()?;

                    for uuid in service_uuids {
                        if uuid == Self::TARGET {
                            guid_tx
                                .send(uuid)
                                .map_err(|_send| windows::core::Error::from_win32())?;
                        }
                    }
                }

                Ok(())
            },
        );
        let token = watcher.Received(&token_handler)?;
        token_tx
            .send(token)
            .map_err(|_e| windows::core::Error::from_win32())?;
        watcher.Start()?;
        Ok(())
    }
}
#[derive(Debug)]
pub enum DeviceQuery {
    Address(u64),
    Id(HSTRING),
}
#[derive(Debug)]
pub enum SearcherSignal {
    FindDevice(DeviceQuery),
}
unsafe impl Send for SearcherSignal {}
pub struct DeviceSearcher {
    input_task: tokio::task::JoinHandle<()>,
    input_tx: tokio::sync::mpsc::UnboundedSender<SearcherSignal>,
    rt: tokio::runtime::Runtime,
}
impl Drop for DeviceSearcher {
    fn drop(&mut self) {
        self.input_task.abort()
    }
}
type StatusVerifier = Box<dyn Fn(bool) + Send>;
type BLStatusHandler = TypedEventHandler<BluetoothLEDevice, IInspectable>;
impl DeviceSearcher {
    const INVOKE: for<'a, 'b> fn(
        &'a Option<BluetoothLEDevice>,
        &'b Option<IInspectable>,
    ) -> windows::core::Result<()> =
        move |devopt: &Option<BluetoothLEDevice>, _args: &Option<IInspectable>| {
            if let Some(device) = devopt {
                let connected: bool = device.ConnectionStatus().ok().map_or(false, |status| {
                    status == BluetoothConnectionStatus::Connected
                });
                println!("Connected: {}", connected);
            };
            Ok(())
        };
    async fn get_device_by_id(deviceid: &HSTRING) -> Option<DeviceWrapper> {
        let Ok(device_future) = BluetoothLEDevice::FromIdAsync(deviceid) else {
            return None;
        };
        let Ok(device) = device_future.await else {
            return None;
        };
        let handler = TypedEventHandler::new(Self::INVOKE);
        let Ok(token) = device.ConnectionStatusChanged(&handler) else {
            return None;
        };
        Some(DeviceWrapper {
            device,
            token,
            services: vec![],
        })
    }
    /// Helper which makes the code smol
    async fn get_device_by_address(bluetoothaddress: u64) -> Option<DeviceWrapper> {
        let Ok(device_future) = BluetoothLEDevice::FromBluetoothAddressAsync(bluetoothaddress)
        else {
            return None;
        };
        let Ok(device) = device_future.await else {
            return None;
        };
        let handler = TypedEventHandler::new(Self::INVOKE);
        let Ok(token) = device.ConnectionStatusChanged(&handler) else {
            return None;
        };
        Some(DeviceWrapper {
            device,
            token,
            services: vec![],
        })
    }
    pub fn new(
        output_tx: tokio::sync::mpsc::UnboundedSender<DeviceWrapper>,
        timeout: Duration,
    ) -> Self {
        let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel();
        let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        let handle = rt.spawn(async move {
            'main: loop {
                println!("Still waiting");
                match input_rx.recv().await {
                    Some(signal) => {
                        println!("DeviceSearcher: {:?}", signal);
                        match signal {
                            SearcherSignal::FindDevice(query) => {
                                let search_start = Instant::now();
                                'search: loop {
                                    if search_start.elapsed() > timeout {
                                        break 'search;
                                    }
                                    let Some(dw) = (match query {
                                        DeviceQuery::Address(bluetoothaddress) => {
                                            Self::get_device_by_address(bluetoothaddress).await
                                        }
                                        DeviceQuery::Id(ref hs) => Self::get_device_by_id(hs).await,
                                    }) else {
                                        println!("nun");
                                        continue 'search;
                                    };
                                    match output_tx.send(dw) {
                                        Ok(_) => println!("Sent"),
                                        _ => {
                                            println!("HELLO WTF");
                                            break 'main;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        println!("Input transmitters gone");
                        break;
                    }
                }
            }
        });
        Self {
            input_task: handle,
            input_tx,
            rt,
        }
    }
    pub fn send_signal(&self, signal: SearcherSignal) -> Result<(), SendError<SearcherSignal>> {
        self.input_tx.send(signal)
    }
}
pub struct DeviceWrapper {
    device: BluetoothLEDevice,
    token: EventRegistrationToken,
    services: Vec<GattDeviceService>,
}
impl DeviceWrapper {}
