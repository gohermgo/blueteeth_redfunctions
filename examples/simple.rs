use std::time::Duration;

use bluetooth_wrapper::DeviceSearcher;
use tokio::sync::mpsc;
fn main() {
    let (device_tx, mut device_rx) = mpsc::unbounded_channel();
    let x = DeviceSearcher::new(device_tx, Duration::from_secs(1));
    x.send_signal(bluetooth_wrapper::SearcherSignal::FindDevice(
        bluetooth_wrapper::DeviceQuery::Address(0),
    ))
    .unwrap();
    let _device = device_rx.blocking_recv().unwrap();
}
