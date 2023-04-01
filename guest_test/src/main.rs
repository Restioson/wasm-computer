use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut eth0 = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/ethernet0")
        .unwrap();
    eth0.write_all(b"Hello!").unwrap();
    Ok(())
}
