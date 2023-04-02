use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::time::Instant;

fn wait_for_file(file: impl AsFd) {
    // Loop to avoid spurious wakeups
    while !host_api::wait_until_ready_for_read(&[file.as_fd()])
        .iter()
        .any(|fd| fd.as_fd().as_raw_fd() == file.as_fd().as_raw_fd())
    {}
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
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/ethernet0")
            .unwrap();
        file.write_all(b"Hello!").unwrap();
        println!("Written 'Hello!' to /dev/ethernet0");
    }

    Ok(())
}
