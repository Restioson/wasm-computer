use anyhow::Result;
use sandboxer::Computer;
use wasmtime::Module;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = sandboxer::our_engine();
    let module = Module::from_file(&engine, "target/wasm32-wasi/debug/guest_test.wasm")?;
    let mut computer = sandboxer::ComputerVm::launch_module(module, Computer::create()?).await?;
    computer.resume().await?;

    Ok(())
}
