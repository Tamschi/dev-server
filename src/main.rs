use {
    dev_server::{serve, Configuration},
    std::{
        collections::HashMap,
        fmt::{Debug, Display, Formatter, Result as fmtResult},
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
    #[structopt(short, long, default_value)]
    port: Port,

    #[structopt(short, long, default_value)]
    remote: Remote,

    #[structopt(short, long, default_value)]
    directory: Directory,

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

    #[structopt(short, long, name = "extension=mime/type...", default_value)]
    content_types: ContentTypes,
}

macro_rules! opt_wrapper {
    ($name:ident($inner:ident) = $default:expr) => {
        #[derive(Debug)]
        struct $name($inner);
        impl Default for $name {
            fn default() -> $name {
                Self($default)
            }
        }
    };
    ($name:ident($inner:ident): Display = $default:expr) => {
        opt_wrapper!($name($inner) = $default);
        opt_wrapper!($name($inner): Display);
    };
    ($name:ident($inner:ident): FromStr = $default:expr) => {
        opt_wrapper!($name($inner) = $default);
        opt_wrapper!($name($inner): FromStr);
    };
    ($name:ident($inner:ident): Display + FromStr = $default:expr) => {
        opt_wrapper!($name($inner) = $default);
        opt_wrapper!($name($inner): Display);
        opt_wrapper!($name($inner): FromStr);
    };
    ($name:ident($inner:ident): Display) => {
        impl Display for $name {
            fn fmt(&self, fmt: &mut Formatter<'_>) -> fmtResult {
                <$inner as Display>::fmt(&self.0, fmt)
            }
        }
    };
    ($name:ident($inner:ident): FromStr) => {
        impl FromStr for $name {
            type Err = <$inner as FromStr>::Err;
            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                $inner::from_str(s).map(Self)
            }
        }
    };
}

opt_wrapper!(Port(u16): Display + FromStr = Configuration::default().endpoint.port());
opt_wrapper!(Remote(IpAddr): Display + FromStr = Configuration::default().endpoint.ip());
opt_wrapper!(Directory(PathBuf): FromStr = Configuration::default().directory.to_owned());

impl Display for Directory {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        <PathBuf as Debug>::fmt(&self.0, fmt)
    }
}

type ContentTypesMap = HashMap<String, String>;
opt_wrapper!(
    ContentTypes(ContentTypesMap) = Configuration::default()
        .content_types
        .iter()
        .map(|c| ((*c.0).to_string(), (*c.1).to_string()))
        .collect()
);

impl FromStr for ContentTypes {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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
        Ok(Self(content_types.collect::<Result<HashMap<_, _>>>()?))
    }
}

impl Display for ContentTypes {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmtResult {
        let mut first = true;
        let mut content_types: Vec<_> = self.0.iter().collect();
        content_types.sort();
        if let Some(max_extension_len) = content_types.iter().map(|c| c.0.len()).max() {
            let fill = fmt.fill();
            for (extension, content_type) in content_types {
                if first {
                    first = false;
                    writeln!(fmt)?;
                }

                // This assumes lower ASCII characters.
                for _ in 0..(max_extension_len - extension.len()) {
                    write!(fmt, "{}", fill)?;
                }
                write!(fmt, "{}={}", extension, content_type)?;
                writeln!(fmt)?;
            }
        }
        Ok(())
    }
}

fn main() {
    simple_logger::init().unwrap();

    let opt = Opt::from_args();
    serve(&Configuration {
        endpoint: SocketAddr::new(opt.remote.0, opt.port.0),
        directory: &opt.directory.0,
        index: &if opt.no_index {
            Vec::new()
        } else {
            opt.index.iter().map(PathBuf::as_path).collect::<Vec<_>>()
        },
        e404: &opt.e404.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        content_types: &opt
            .content_types
            .0
            .iter()
            .map(|c| (c.0.as_ref(), c.1.as_ref()))
            .collect::<HashMap<_, _>>(),
    })
    .unwrap();
}
