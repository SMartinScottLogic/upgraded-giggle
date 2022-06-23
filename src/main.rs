use std::ffi::{OsStr, OsString};
use std::{env, io};

extern crate log;

fn main() -> io::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let args: Vec<OsString> = env::args_os().collect();

    if args.len() != 5 {
        println!("usage: {} <server> <username/email> <password> <mountpoint>", &env::args().next().unwrap());
        ::std::process::exit(-1);
    }
    
    let (server, username, password, mountpoint) = (&args[1], &args[2], &args[3], &args[4]);

    let filesystem =
        upgraded_giggle::SeafileFS::new(server, username, password);
    let options = ["-o", "rw", "-o", "fsname=seafile", "-a", "auto_mount"];
    let options = options.iter().map(|o| o.as_ref()).collect::<Vec<&OsStr>>();
    fuse_mt::mount(fuse_mt::FuseMT::new(filesystem, 1), &mountpoint, &options)
}
