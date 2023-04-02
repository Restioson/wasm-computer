pub mod devices;
mod host_api;

use crate::devices::{virtual_fs::DevicesDir, Devices, AttachedDuplexLink};
use anyhow::Result;
use std::collections::VecDeque;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use utf8::BufReadDecoder;
use uuid::Uuid;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime::{Config, Engine, Func, Linker, Module, Store, Val};
use wasmtime_wasi::sync::WasiCtxBuilder;
use wasmtime_wasi::{ambient_authority, Dir, WasiCtx};

pub fn our_engine() -> Engine {
    Engine::new(
        Config::new().async_support(true), // .epoch_interruption(true) // TODO epoch interruption
    )
    .unwrap()
}

pub struct Computer {
    id: Uuid,
    devices: Devices,
}

impl Computer {
    /// Create a brand new computer with a new id
    pub fn create() -> Result<Computer> {
        let computer = Computer {
            id: Uuid::new_v4(),
            devices: Devices::new(),
        };

        std::fs::create_dir_all(computer.home_dir())?;

        Ok(computer)
    }

    pub fn devices_mut(&mut self) -> &mut Devices {
        &mut self.devices
    }

    pub fn root_dir(&self) -> PathBuf {
        let path = Path::new("out/computers");
        path.join(self.id.to_string())
    }

    pub fn home_dir(&self) -> PathBuf {
        let mut path = self.root_dir();
        path.push("home");
        path.push("alex");
        path
    }
}

pub struct ComputerVmState {
    wasi: WasiCtx,
    stdout: Arc<RwLock<VecDeque<u8>>>,
    stderr: Arc<RwLock<VecDeque<u8>>>,
    stdin: Arc<RwLock<VecDeque<u8>>>,
    computer: Arc<RwLock<Computer>>,
}

impl ComputerVmState {
    fn new(computer: Computer) -> Result<Self> {
        let stdout = Arc::new(RwLock::new(VecDeque::new()));
        let stderr = Arc::new(RwLock::new(VecDeque::new()));
        let stdin = Arc::new(RwLock::new(VecDeque::new()));

        let wasi = WasiCtxBuilder::new()
            .stdout(Box::new(WritePipe::from_shared(stdout.clone())))
            .stderr(Box::new(WritePipe::from_shared(stderr.clone())))
            .stdin(Box::new(ReadPipe::from_shared(stdin.clone())))
            .preopened_dir(
                Dir::open_ambient_dir(computer.root_dir(), ambient_authority())?,
                "/",
            )?
            .preopened_dir(
                Dir::open_ambient_dir(computer.home_dir(), ambient_authority())?,
                ".",
            )?
            .env("RUST_BACKTRACE", "full")?
            .build();

        let computer = Arc::new(RwLock::new(computer));

        wasi.push_preopened_dir(
            Box::new(DevicesDir::new(computer.clone())),
            PathBuf::from("/dev/"),
        )?;

        Ok(ComputerVmState {
            wasi,
            stdout,
            stderr,
            stdin,
            computer,
        })
    }
}

pub struct ComputerVm {
    main_thread: Func,
    store: Store<ComputerVmState>,
}

impl ComputerVm {
    pub async fn launch_module(module: Module, computer: Computer, arg: &str) -> Result<ComputerVm> {
        let mut store = Store::new(module.engine(), ComputerVmState::new(computer)?);
        // store.epoch_deadline_async_yield_and_update(100); // TODO epoch interruption

        // TODO: reuse linker
        let mut linker = Linker::new(module.engine());
        wasmtime_wasi::add_to_linker(&mut linker, |s: &mut ComputerVmState| &mut s.wasi)?;
        host_api::add_exports(&mut linker)?;
        linker.module_async(&mut store, "", &module).await?;

        store.data_mut().wasi.push_arg(arg)?;

        let main_func = linker.get_default(&mut store, "")?;

        Ok(ComputerVm {
            main_thread: main_func,
            store,
        })
    }

    pub fn add_ethernet(&mut self, link: AttachedDuplexLink) {
        self.store.data_mut().computer.write().unwrap().devices_mut().add_ethernet(link)
    }

    pub async fn resume(&mut self) -> Result<()> {
        let ty = self.main_thread.ty(&mut self.store);
        let mut results = vec![Val::null(); ty.results().len()];
        let res = self
            .main_thread
            .call_async(&mut self.store, &[], &mut results)
            .await;

        let mut stdout = self.store.data().stdout.write().unwrap();
        let stdout = BufReadDecoder::read_to_string_lossy(BufReader::new(&mut *stdout)).unwrap();
        println!("Stdout: {stdout}");

        let mut stderr = self.store.data().stderr.write().unwrap();
        let stderr = BufReadDecoder::read_to_string_lossy(BufReader::new(&mut *stderr)).unwrap();
        println!("Stderr: {stderr}");

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.is::<wasmtime::Trap>() {
                    println!("Trapped: {:?}", e.downcast_ref::<wasmtime::Trap>().unwrap());
                }
                Err(e)
            }
        }
    }
}
