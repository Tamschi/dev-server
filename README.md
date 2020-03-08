# dev-server

The bare minimum of what I need for developing statically hosted sites locally.

```sh
cargo install --git https://github.com/Tamschi/dev-server.git
dev-server --help
```

```text
dev-server 0.0.0
Tamme Schichler <tamme@schichler.dev>

A simple development HTTP server, focusing on simplicity and secure defaults.

¹   some/path => relative to the served directory
    ./some/path => relative to the requested path

    If multiple paths are given, they are tried in order.

No files outside the served directory are served.

USAGE:
    dev-server.exe [FLAGS] [OPTIONS]

FLAGS:
    -h, --help        Prints help information
        --no-index    disables --index
    -V, --version     Prints version information

OPTIONS:
    -d, --directory <directory>                      [default: "."]
    -c, --content-types <extension=mime/type...>
             [default:
             css=text/css
            html=text/html
              js=text/javascript
            wasm=application/wasm
            ]
    -i, --index <index>...                          ¹ [default: ./index.html]
        --404 <path>...                             ¹
    -p, --port <port>                                [default: 8000]
    -r, --remote <remote>                            [default: 127.0.0.1]
```

```sh
dev-server -d target/bundle/schichler-dev --404 404.html
```

TODO: Redo this at a sane hour.

```log
2020-03-08 01:44:24,865 INFO  [dev_server] Serving "C:\\[...]\\schichler-dev"...
2020-03-08 01:44:28,102 INFO  [dev_server] "" -> index "./index.html"
2020-03-08 01:44:28,336 INFO  [dev_server] "semantic.min.css" -> file "semantic.min.css"
2020-03-08 01:44:28,337 INFO  [dev_server] "style.css" -> file "style.css"
2020-03-08 01:44:28,340 INFO  [dev_server] "schichler-dev.js" -> file "schichler-dev.js"
2020-03-08 01:44:28,472 INFO  [dev_server] "schichler-dev_bg.wasm" -> file "schichler-dev_bg.wasm"
2020-03-08 01:44:29,624 INFO  [dev_server] "themes/default/assets/fonts/outline-icons.woff2" -> file "themes/default/assets/fonts/outline-icons.woff2"
2020-03-08 01:44:29,628 INFO  [dev_server] "themes/default/assets/fonts/brand-icons.woff2" -> file "themes/default/assets/fonts/brand-icons.woff2"
2020-03-08 01:44:29,656 WARN  [dev_server] "favicon.ico" -> 404 "404.html"
```

## Q&A

Q: Is this HTTP-compliant?  
A: Probably not. It's (just barely) enough for Firefox to display my homepage project, but Edgium complains about connection resets.

Q: Is this fast?  
A: It's a single-threaded web server without cache using HTTP 1.0 without keep-alive. No.

Q: Is this secure?  
A: Yes, it should be! The server never writes to disk and doesn't contain any unsafe Rust code. Additionally, the server doesn't serve or check files outside the specified directory (URLs with `..` parts are discarded early) and no external port is exposed by default. If the latter is necessary, you can use `--remote 0.0.0.0` to open to all IPv4 endpoints. (IPv6 addresses should work equally well.)

## Library (`dev_server`)

This crate contains a library target that exposes a `struct Configuration` (for convenient `Default`s and named parameters) and a single `serve` method that won't return until an error happens.

The library will follow Semver **after** leaving version `0.0.0`.
