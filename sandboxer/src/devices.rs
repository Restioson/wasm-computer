pub mod virtual_fs;

use crate::ComputerVmState;
use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Poll, Waker};
use event_listener::{Event, EventListener};
use futures::future::Either;
use wasmtime::{AsContextMut, Func};

#[derive(Default)]
struct Buffer {
    buf: VecDeque<u8>,
    on_send: Event,
}

#[derive(Default)]
struct DuplexLink {
    // TODO: inode, device id
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

struct Notifier {
    waker: Waker,
}

pub struct Devices {
    ethernet_links: Vec<AttachedDuplexLink>,
    wireless_links: Vec<AttachedDuplexLink>,
}

impl Devices {
    pub fn new() -> Devices {
        Devices {
            ethernet_links: vec![],
            wireless_links: vec![],
        }
    }

    pub fn add_ethernet(&mut self, link: AttachedDuplexLink) {
        self.ethernet_links.push(link);
    }

    pub fn is_ready_for_read(&self, device_id: u64) -> bool {
        self.ethernet_links[device_id as usize].read_buf().buf.len() > 0
    }

    pub fn wait_until_ready_for_read(&self, device_id: u64) -> impl Future<Output = ()> + Unpin {
        let listener = self.ethernet_links[device_id as usize].read_buf().on_send.listen();
        if !self.is_ready_for_read(device_id) {
            Either::Left(listener)
        } else {
            Either::Right(futures::future::ready(()))
        }
    }
}
