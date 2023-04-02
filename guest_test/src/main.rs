use std::fs::File;
use std::os::fd::AsRawFd;
use host_api::{Interest, InterestFlags, Ready};
use bytemuck::Zeroable;
use std::io::{Write, Read};
use std::time::Instant;

fn wait_for_file(file: &File) {
    let interests = [
        Interest {
            fd: file.as_raw_fd() as u32,
            interest_flags: InterestFlags::READ.bits(),
        },
    ];
    let mut ready = [Ready::zeroed(); 1];
    host_api::call_host::notify_ready(&interests, &mut ready);
}

fn main() -> std::io::Result<()> {
    if std::env::args().next().unwrap() == "1" {
        let mut file = File::open("/dev/ethernet0").unwrap();
        println!("Waiting for data on /dev/ethernet0...");
        let time = Instant::now();
        wait_for_file(&file);

        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        println!("Got '{}' after {}ms", s, time.elapsed().as_millis());
    } else {
        let mut file = std::fs::OpenOptions::new().write(true).open("/dev/ethernet0").unwrap();
        file.write(b"Hello!").unwrap();
        println!("Written 'Hello!' to /dev/ethernet0");
    }

    Ok(())
}
