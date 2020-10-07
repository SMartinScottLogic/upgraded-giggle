use std::ffi::{OsStr, OsString};
use std::{env, io};

mod seafile;

#[macro_use]
extern crate log;

fn main() -> io::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let args: Vec<OsString> = env::args_os().collect();

    if args.len() != 2 {
        println!("usage: {} <mountpoint>", &env::args().next().unwrap());
        ::std::process::exit(-1);
    }

    let filesystem =
        seafile::SeafileFS::new("http://192.168.0.32", "havvoric@gmail.com", "Alpha3wyrd");
    let options = ["-o", "rw", "-o", &"fsname=seafile", "-a", "auto_mount"];
    let options = options.iter().map(|o| o.as_ref()).collect::<Vec<&OsStr>>();
    fuse_mt::mount(fuse_mt::FuseMT::new(filesystem, 1), &args[1], &options)
}
