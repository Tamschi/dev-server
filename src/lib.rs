#![allow(clippy::cognitive_complexity)]
//#![warn(missing_docs)]

//! A small development server, just enough for a static page from a single directory.
//! The defaults ensure secure limits regarding what is served to where.

use {
    dunce::realpath,
    lazy_static::lazy_static,
    log::{error, info, warn},
    std::{
        collections::{hash_map::RandomState, HashMap},
        fs::File,
        hash::BuildHasher,
        io::{copy, Error, ErrorKind, Read, Result, Write},
        net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
        path::{Path, PathBuf},
    },
};

#[doc(hidden)]
pub enum Never {}

pub struct Configuration<'a, E: ToSocketAddrs = SocketAddr, S: BuildHasher = RandomState> {
    pub endpoint: E,
    pub directory: &'a Path,
    pub index: &'a [&'a Path],
    pub e404: &'a [&'a Path],
    pub content_types: &'a HashMap<&'a str, &'a str, S>,
}

lazy_static! {
    static ref DEFAULT_DIRECTORY: &'static Path =
        Box::leak(Box::new(PathBuf::from(".",))).as_path();
    static ref DEFAULT_INDEX: &'static [&'static Path] = &*Box::leak(Box::new([Box::leak(
        Box::new(PathBuf::from("./index.html",))
    )
    .as_path()]));
    static ref DEFAULT_CONTENT_TYPES: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("html", "text/html");
        map.insert("css", "text/css");
        map.insert("js", "text/javascript");
        map.insert("wasm", "application/wasm");
        map
    };
}

impl Default for Configuration<'_, SocketAddr, RandomState> {
    #[inline]
    fn default() -> Self {
        Self {
            endpoint: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8000),
            directory: &DEFAULT_DIRECTORY,
            index: &DEFAULT_INDEX,
            e404: &[],
            content_types: &DEFAULT_CONTENT_TYPES,
        }
    }
}

pub fn serve<E: ToSocketAddrs, S: ::std::hash::BuildHasher>(
    configuration: &Configuration<E, S>,
) -> Result<Never> {
    let Configuration {
        endpoint,
        directory,

        index,
        e404,
        content_types,
    } = configuration;
    let directory = realpath(directory)?;
    info!("Serving {:?}...", &directory);
    for incoming in TcpListener::bind(endpoint)?.incoming() {
        match incoming {
            Ok(incoming) => {
                if let Err(error) = handle_request(incoming, &directory, index, e404, content_types)
                {
                    error!("{}", error)
                }
            }
            Err(error) => error!("{}", error),
        }
    }
    unreachable!()
}

fn handle_request<S: ::std::hash::BuildHasher>(
    mut stream: TcpStream,
    directory: &Path,
    index: &[&Path],
    e404: &[&Path],
    content_types: &HashMap<&str, &str, S>,
) -> Result<()> {
    let request = {
        //BAD
        let mut buffer: Vec<u8> = Vec::new();
        let mut buf = [0];
        while let Ok(1) = stream.read(&mut buf) {
            buffer.extend(buf.iter().copied());
            if buf[0] == b'\r' || buf[0] == b'\n' {
                break;
            }
        }
        if buffer.last() != Some(&b'\r') && buffer.last() != Some(&b'\n') {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "No newline found in request",
            ));
        }
        String::from_utf8_lossy(&buffer).to_string()
    };
    let mut split = request.split(' ');
    match split.next() {
        None => unreachable!(),
        Some("GET") => {
            let path = split
                .next()
                .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "No request path found"))?;

            if !path.starts_with('/') {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Paths must be absolute",
                ));
            }

            let is_index_path = path.ends_with('/');

            let path: PathBuf = path.chars().skip(1).collect::<String>().into();

            if path.has_root() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Paths must not start with //",
                ));
            }

            for part in path.iter() {
                if part == ".." {
                    stream.write_all(
                        b"HTTP/1.0 403 Forbidden\r\n\r\ndev-server doesn't support .. in URLs.",
                    )?;
                    stream.flush()?;
                    warn!("{:?} -> .. forbidden", path);
                    stream.shutdown(Shutdown::Write)?;
                    return Ok(());
                }
            }

            if is_index_path {
                for index in index.iter() {
                    let index = if index.starts_with(".") {
                        path.join(index)
                    } else {
                        index.into()
                    };
                    if try_serve(&mut stream, b"200 OK", &directory, &index, content_types).is_ok()
                    {
                        info!("{:?} -> index {:?}", path, index);
                        stream.shutdown(Shutdown::Write)?;
                        return Ok(());
                    }
                }
            } else if try_serve(&mut stream, b"200 OK", &directory, &path, content_types).is_ok() {
                info!("{:?} -> file {:?}", path, path);
                stream.shutdown(Shutdown::Write)?;
                return Ok(());
            }

            for e404 in e404.iter() {
                let e404 = if e404.starts_with(".") {
                    path.join(e404)
                } else {
                    e404.into()
                };
                if try_serve(
                    &mut stream,
                    b"404 Not Found",
                    &directory,
                    &e404,
                    content_types,
                )
                .is_ok()
                {
                    warn!("{:?} -> 404 {:?}", path, e404);
                    stream.shutdown(Shutdown::Write)?;
                    return Ok(());
                }
            }
            stream.write_all(b"HTTP/1.0 404 Not Found\r\n\r\n")?;
            stream.flush()?;
            error!("{:?} -> 404", path);
            stream.shutdown(Shutdown::Write)?;
            return Ok(());
        }
        method => warn!("Unhandled request: {:?} {:?}", method, split.next()),
    }
    Ok(())
}

fn try_serve<S: ::std::hash::BuildHasher>(
    stream: &mut impl Write,
    status: &[u8],
    directory: &Path,
    file: &Path,
    content_types: &HashMap<&str, &str, S>,
) -> Result<()> {
    let file = realpath(directory.join(file))?;
    if !file.starts_with(directory) {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "Can't serve: Outside directory",
        ));
    }
    let content_type = file
        .extension()
        .map(|ext| ext.to_string_lossy())
        .map(|ext| content_types.get(ext.as_ref()))
        .flatten();
    let mut file = File::open(file)?;
    stream.write_all(b"HTTP/1.0 ")?;
    stream.write_all(status)?;
    if let Some(content_type) = content_type {
        stream.write_all(b"\r\nContent-Type: ")?;
        stream.write_all(content_type.as_bytes())?;
    }
    stream.write_all(b"\r\n\r\n")?;
    copy(&mut file, stream)?;
    stream.flush()?;
    Ok(())
}
