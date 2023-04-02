use crate::ComputerVmState;
use anyhow::Result;
use wasi_common::snapshots::preview_1::types::Fd;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;
use wasmtime::Caller;
use wasmtime::Linker;

pub fn add_exports(linker: &mut Linker<ComputerVmState>) -> Result<()> {
    linker.func_wrap3_async("event", "wait_until_ready", device::wait_until_ready)?;
    Ok(())
}

mod device {
    use super::*;
    use crate::devices::virtual_fs::decompose_device;
    use anyhow::Context;
    use futures::future::Either;
    use host_api_sys::{Interest, Ready};
    use std::future::Future;
    use wasmtime::Extern;

    pub fn wait_until_ready<'a>(
        mut caller: Caller<'a, ComputerVmState>,
        interests_ptr: i64,
        ready_ptr: i64,
        len: i64,
    ) -> Box<dyn Future<Output = Result<i64>> + Send + 'a> {
        Box::new(async move {
            let mem = match caller.get_export("memory") {
                Some(Extern::Memory(mem)) => mem,
                _ => anyhow::bail!("failed to find host memory"),
            };

            let interests = {
                let interests_bytes = mem
                    .data(&caller)
                    .get(
                        interests_ptr as usize
                            ..interests_ptr as usize
                                + (len as usize * std::mem::size_of::<Interest>()),
                    )
                    .context("failed to load interests")?;

                let interests_guest: &[Interest] = bytemuck::cast_slice(interests_bytes);
                Vec::from(interests_guest)
            };

            let mut devices = Vec::with_capacity(interests.len());

            {
                let vm = caller.data_mut();
                for interest in &interests {
                    let dev = vm.wasi.fd_filestat_get(Fd::from(interest.fd)).await?.dev;
                    let computer = vm.computer.read().unwrap();

                    match decompose_device(dev) {
                        Some((dev_type, dev_idx))
                            if !computer.devices.contains(dev_type, dev_idx) =>
                        {
                            anyhow::bail!("Unknown device: type {dev_type:?}, idx {dev_idx}")
                        }
                        _ => (),
                    }

                    devices.push(dev);
                }
            }

            // TODO check that it is correct fd
            let wait = devices.iter().map(|device| {
                let computer = caller.data().computer.read().unwrap();
                match decompose_device(*device) {
                    // Is a device managed by /dev/
                    Some((dev_type, dev_idx)) => Either::Left(
                        computer
                            .devices
                            .wait_until_ready_for_read(dev_type, dev_idx)
                            .unwrap(),
                    ),
                    // Is a regular file, so it is always ready for read
                    None => Either::Right(futures::future::ready(())),
                }
            });

            futures::future::select_all(wait).await;

            let ready: Vec<Ready> = {
                let computer = caller.data().computer.read().unwrap();
                devices
                    .iter()
                    .zip(interests.iter())
                    .filter(|(dev, _interest)| {
                        match decompose_device(**dev) {
                            // Is a device managed by /dev/
                            Some((dev_type, dev_idx)) => computer
                                .devices
                                .is_ready_for_read(dev_type, dev_idx)
                                .unwrap(),
                            // Is a regular file, so it is always ready for read
                            None => true,
                        }
                    })
                    .map(|(_, interest)| Ready {
                        fd: interest.fd,
                        interest_flags: interest.interest_flags,
                    })
                    .collect()
            };

            let ready_bytes = mem
                .data_mut(&mut caller)
                .get_mut(
                    ready_ptr as usize
                        ..ready_ptr as usize + (len as usize * std::mem::size_of::<Ready>()),
                )
                .context("failed to load ready array")?;
            let ready_guest: &mut [Ready] = bytemuck::cast_slice_mut(ready_bytes);

            let ready_len = ready_guest
                .iter_mut()
                .zip(ready)
                .map(|(slot, ready_dev)| *slot = ready_dev)
                .count();

            Ok(ready_len as i64)
        })
    }
}
