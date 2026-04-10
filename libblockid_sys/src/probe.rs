use libblockid_core::{BlockInfo, Filter, LowProbe};

use crate::{
    error::{Error, ErrorKind},
    io::File,
    path::{Path, PathBuf, SysPath},
    topology::TopologyInfo,
};

struct Probe {
    path: PathBuf,
    disk: File,
}

impl Probe {
    pub fn open<P: SysPath>(path: P) -> Result<Probe, Error> {
        let file = File::open(&path)?;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            disk: file,
        })
    }

    pub fn probe_info(&mut self, offset: u64, filter: Filter) -> Result<BlockInfo, Error> {
        let mut low_probe = LowProbe::new(&mut self.disk, offset, filter);

        let info = low_probe.probe();

        todo!()
    }

    pub fn probe_topology(&self) -> Result<TopologyInfo, Error> {
        todo!()
    }
}
