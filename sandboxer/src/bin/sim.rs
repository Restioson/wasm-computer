use anyhow::Result;
use sandboxer::devices::AttachedDuplexLink;
use sandboxer::Computer;
use std::time::Duration;
use wasmtime::Module;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = sandboxer::our_engine();
    let module = Module::from_file(&engine, "target/wasm32-wasi/debug/guest_test.wasm")?;
    let mut computer1 =
        sandboxer::ComputerVm::launch_module(module.clone(), Computer::create()?, "1").await?;
    let mut computer2 =
        sandboxer::ComputerVm::launch_module(module, Computer::create()?, "2").await?;

    let (link1, link2) = AttachedDuplexLink::new_pair();
    computer1.add_ethernet(link1);
    computer2.add_ethernet(link2);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        computer2.resume().await.unwrap()
    });

    computer1.resume().await?;

    Ok(())
}
