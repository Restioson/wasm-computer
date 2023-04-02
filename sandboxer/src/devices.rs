pub mod virtual_fs;

use event_listener::Event;
use futures::future::Either;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Default)]
// TODO buffer full
struct Buffer {
    buf: VecDeque<u8>,
    on_send: Event,
}

#[derive(Default)]
struct DuplexLink {
    duplex_bufs: [Mutex<Buffer>; 2],
}

#[derive(Clone)]
pub struct AttachedDuplexLink {
    first_half: bool,
    shared: Arc<DuplexLink>,
}

impl AttachedDuplexLink {
    pub fn new_pair() -> (AttachedDuplexLink, AttachedDuplexLink) {
        let shared = Arc::new(DuplexLink::default());

        let first = AttachedDuplexLink {
            first_half: true,
            shared: shared.clone(),
        };
        let second = AttachedDuplexLink {
            first_half: false,
            shared,
        };

        (first, second)
    }

    fn read_buf(&self) -> MutexGuard<'_, Buffer> {
        if self.first_half {
            self.shared.duplex_bufs[1].lock().unwrap()
        } else {
            self.shared.duplex_bufs[0].lock().unwrap()
        }
    }

    fn write_buf(&self) -> MutexGuard<'_, Buffer> {
        if self.first_half {
            self.shared.duplex_bufs[0].lock().unwrap()
        } else {
            self.shared.duplex_bufs[1].lock().unwrap()
        }
    }
}

#[derive(Default)]
pub struct Devices {
    ethernet_links: Vec<AttachedDuplexLink>,
    wireless_links: Vec<AttachedDuplexLink>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeviceType {
    Ethernet,
    Wireless,
}

impl Devices {
    pub fn add_ethernet(&mut self, link: AttachedDuplexLink) {
        self.ethernet_links.push(link);
    }

    fn device(&self, dev_type: DeviceType, dev_idx: usize) -> Option<&AttachedDuplexLink> {
        match dev_type {
            DeviceType::Ethernet => self.ethernet_links.get(dev_idx),
            DeviceType::Wireless => self.wireless_links.get(dev_idx),
        }
    }

    pub fn contains(&self, dev_type: DeviceType, dev_idx: usize) -> bool {
        self.device(dev_type, dev_idx).is_some()
    }

    pub fn is_ready_for_read(&self, dev_type: DeviceType, dev_idx: usize) -> Option<bool> {
        self.device(dev_type, dev_idx)
            .map(|dev| !dev.read_buf().buf.is_empty())
    }

    pub fn wait_until_ready_for_read(
        &self,
        dev_type: DeviceType,
        dev_idx: usize,
    ) -> Option<impl Future<Output = ()> + Unpin> {
        let dev = self.device(dev_type, dev_idx)?;
        let listener = dev.read_buf().on_send.listen();

        Some(if dev.read_buf().buf.is_empty() {
            Either::Left(listener)
        } else {
            Either::Right(futures::future::ready(()))
        })
    }
}
