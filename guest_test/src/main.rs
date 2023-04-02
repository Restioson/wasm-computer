use std::os::fd::AsRawFd;
use host_api::{Interest, InterestFlags, Ready};
use bytemuck::Zeroable;
use std::io::{Write, Read};

fn main() -> std::io::Result<()> {
    if std::env::args().next().unwrap() == "1" {
        let mut file = std::fs::File::open("/dev/ethernet0").unwrap();
        let interests = [
            Interest {
                fd: file.as_raw_fd() as u32,
                interest_flags: InterestFlags::READ.bits(),
            },
        ];

        let mut ready = [Ready::zeroed(); 1];

        host_api::call_host::notify_ready(&interests, &mut ready);
        println!("{:?}", ready);

        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        println!("Got: '{}'", s);
    } else {
        let mut file = std::fs::OpenOptions::new().write(true).open("/dev/ethernet0").unwrap();
        file.write(b"Hello!").unwrap();
    }

    Ok(())
}
