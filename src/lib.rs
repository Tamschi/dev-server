#![allow(clippy::cognitive_complexity)]

use {
    dunce::realpath,
    log::{error, info, warn},
    std::{
        fs::File,
        io::{copy, Error, ErrorKind, Read, Result, Write},
        net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs},
        path::{Path, PathBuf},
    },
};

pub enum Never {}
pub fn serve(
    endpoint: impl ToSocketAddrs,
    directory: &Path,
    index: &[&Path],
    e404: &[&Path],
) -> Result<Never> {
    let directory = realpath(directory)?;
    info!("Serving {:?}...", &directory);
    for incoming in TcpListener::bind(endpoint)?.incoming() {
        if let Err(error) = handle_incoming(incoming, &directory, index, e404) {
            error!("{}", error)
        }
    }
    unreachable!()
}

pub fn handle_incoming(
    incoming: Result<TcpStream>,
    directory: &Path,
    index: &[&Path],
    e404: &[&Path],
) -> Result<()> {
    let mut stream = incoming?;

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
                    if try_serve(&mut stream, b"200 OK", &directory, &index).is_ok() {
                        info!("{:?} -> index {:?}", path, index);
                        stream.shutdown(Shutdown::Write)?;
                        return Ok(());
                    }
                }
            } else if try_serve(&mut stream, b"200 OK", &directory, &path).is_ok() {
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
                if try_serve(&mut stream, b"404 Not Found", &directory, &e404).is_ok() {
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

fn try_serve(stream: &mut impl Write, status: &[u8], directory: &Path, file: &Path) -> Result<()> {
    let file = realpath(directory.join(file))?;
    if !file.starts_with(directory) {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "Can't serve: Outside directory",
        ));
    }
    let mut file = File::open(file)?;
    stream.write_all(b"HTTP/1.0 ")?;
    stream.write_all(status)?;
    stream.write_all(b"\r\n\r\n")?;
    copy(&mut file, stream)?;
    stream.flush()?;
    Ok(())
}
