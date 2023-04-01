pub mod virtual_fs;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

struct DuplexLink {
    // TODO: inode, device id
    duplex_bufs: [Mutex<VecDeque<u8>>; 2],
}

impl DuplexLink {
    fn new() -> DuplexLink {
        DuplexLink {
            duplex_bufs: [Mutex::new(VecDeque::new()), Mutex::new(VecDeque::new())],
        }
    }
}

#[derive(Clone)]
struct AttachedDuplexLink {
    first_half: bool,
    shared: Arc<DuplexLink>,
}

impl AttachedDuplexLink {
    /// One-sided attached duplex link that writes into the void
    fn new_sink() -> AttachedDuplexLink {
        AttachedDuplexLink {
            first_half: true,
            shared: Arc::new(DuplexLink::new()),
        }
    }

    fn new_pair() -> (AttachedDuplexLink, AttachedDuplexLink) {
        let shared = Arc::new(DuplexLink::new());

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

    fn read_buf(&self) -> MutexGuard<'_, VecDeque<u8>> {
        if self.first_half {
            self.shared.duplex_bufs[1].lock().unwrap()
        } else {
            self.shared.duplex_bufs[0].lock().unwrap()
        }
    }

    fn write_buf(&self) -> MutexGuard<'_, VecDeque<u8>> {
        if self.first_half {
            self.shared.duplex_bufs[0].lock().unwrap()
        } else {
            self.shared.duplex_bufs[1].lock().unwrap()
        }
    }
}

pub struct Devices {
    ethernet_links: Vec<AttachedDuplexLink>,
    wireless_links: Vec<AttachedDuplexLink>,
}

impl Devices {
    pub fn new_sink() -> Self {
        Devices {
            ethernet_links: vec![
                AttachedDuplexLink::new_sink(),
                AttachedDuplexLink::new_sink(),
            ],
            wireless_links: vec![AttachedDuplexLink::new_sink()],
        }
    }
}
