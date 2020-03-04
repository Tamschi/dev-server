use {
    dev_server::serve,
    std::{
        net::{IpAddr, SocketAddr},
        path::PathBuf,
    },
    structopt::StructOpt,
};

//TODO?: ..*/some/path => relative to the requested path, but also recursively try parent directories
#[derive(Debug, StructOpt)]
#[structopt(
    author,
    about = "
A simple development HTTP server, focusing on simplicity and secure defaults.

¹   some/path => relative to the served directory
    ./some/path => relative to the requested path

    If multiple paths are given, they are tried in order.

No files outside the served directory are served."
)]
struct Opt {
    #[structopt(short, long, default_value = "8000")]
    port: u16,

    #[structopt(short, long, default_value = "127.0.0.1")]
    remote: IpAddr,

    #[structopt(short, long, parse(from_os_str), default_value = ".")]
    directory: PathBuf,

    #[structopt(
        short,
        long,
        parse(from_os_str),
        default_value = "./index.html",
        help = "¹"
    )]
    index: Vec<PathBuf>,
    #[structopt(long, help = "disables --index")]
    no_index: bool,

    #[structopt(long = "404", name = "path", parse(from_os_str), help = "¹")]
    e404: Vec<PathBuf>,
}

fn main() {
    simple_logger::init().unwrap();

    let opt = Opt::from_args();
    serve(
        SocketAddr::new(opt.remote, opt.port),
        &opt.directory,
        &if opt.no_index {
            Vec::new()
        } else {
            opt.index.iter().map(PathBuf::as_path).collect::<Vec<_>>()
        },
        &opt.e404.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
    )
    .unwrap();
}
