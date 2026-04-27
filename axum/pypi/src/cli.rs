use clap::{Arg, Command, command};
use std::net::Ipv4Addr;
use std::path::PathBuf;

pub fn build() -> Command {
    let packages = Arg::new("PACKAGES")
        .long("packages")
        .value_parser(clap::value_parser!(PathBuf))
        .default_value("packages")
        .action(clap::ArgAction::Set);

    let host = Arg::new("HOST")
        .long("host")
        .value_parser(clap::value_parser!(Ipv4Addr))
        .default_value("0.0.0.0")
        .action(clap::ArgAction::Set);

    let port = Arg::new("PORT")
        .long("port")
        .value_parser(clap::value_parser!(u16))
        .default_value("8080")
        .action(clap::ArgAction::Set);

    let hash = Arg::new("HASH")
        .long("hash")
        .value_parser(["md5", "sha256"])
        .default_value("sha256")
        .action(clap::ArgAction::Set);

    let debug = Arg::new("DEBUG")
        .long("debug")
        .action(clap::ArgAction::SetTrue);

    command!()
        .arg(packages)
        .arg(host)
        .arg(port)
        .arg(hash)
        .arg(debug)
}
