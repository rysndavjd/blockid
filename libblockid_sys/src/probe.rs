use crate::{
    error::{Error, ErrorKind},
    io::File,
    path::SysPath,
};

struct Probe<P: SysPath> {
    path: P,
    disk: File,
    
}

impl<P: SysPath> Probe<P> {
    fn new(path: P) -> Result<Probe<P>, Error> {
        let file = File::open(path)?;

        todo!()
    }
}
