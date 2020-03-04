use {
    dev_server::serve,
    std::{
        collections::HashMap,
        io::{Error, ErrorKind, Result},
        net::{IpAddr, SocketAddr},
        path::PathBuf,
        str::FromStr,
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

    #[structopt(short, long, name = "extension=mime/type", default_value)]
    content_types: Wrapper<Vec<(String, String)>>,
}

#[derive(Debug)]
struct Wrapper<T>(T);
impl<T> Wrapper<T> {
    fn unwrap(self) -> T {
        self.0
    }
}

impl FromStr for Wrapper<Vec<(String, String)>> {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let content_types = s.split_whitespace().filter(|s| !s.is_empty()).map(|s| {
            let mut split = s.split('=');
            let extension = split.next().unwrap();
            let mime = split.next().ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, format!("No = found in {:?}", s))
            })?;
            if split.next().is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Too many = found in {:?}", s),
                ));
            }
            Ok((extension.to_owned(), mime.to_owned()))
        });
        Ok(Self(content_types.collect::<Result<Vec<_>>>()?))
    }
}
impl ToString for Wrapper<Vec<(String, String)>> {
    fn to_string(&self) -> String {
        let pairs: Vec<_> = self
            .0
            .iter()
            .map(|c| [c.0.as_ref(), c.1.as_ref()].join("="))
            .collect();
        pairs[..].join(" ")
    }
}

impl Default for Wrapper<Vec<(String, String)>> {
    fn default() -> Self {
        Self(vec![
            ("html".to_owned(), "text/html".to_owned()),
            ("css".to_owned(), "text/css".to_owned()),
            ("js".to_owned(), "text/javascript".to_owned()),
            ("wasm".to_owned(), "application/wasm".to_owned()),
        ])
    }
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
        &opt.content_types
            .unwrap()
            .into_iter()
            .collect::<HashMap<_, _>>(),
    )
    .unwrap();
}
