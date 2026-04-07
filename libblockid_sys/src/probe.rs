use crate::{io::File, path::Arg};

struct Probe<P: Arg> {
    path: P,
    disk: File,
}
