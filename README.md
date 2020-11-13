# fLotte User Management Server

![Build and Test](https://github.com/fLotte-meets-HWR-DB/flotte-user-management/workflows/Build%20and%20Test/badge.svg)
![Cargo Audit Check](https://github.com/fLotte-meets-HWR-DB/flotte-user-management/workflows/Run%20Cargo%20Audit/badge.svg)
![Build Docker Image](https://github.com/fLotte-meets-HWR-DB/flotte-user-management/workflows/Build%20Docker%20Image/badge.svg)

## Dev Requirements

- a full rust toolchain installation (for example with [rustup](https://rustup.rs/))
- a postgres installation

## Building

```sh
# in the projects folder
> cargo build --release
```

The resulting binary is being stored in the `target` folder.


## Running

The server can be run either directly from the built binary or with the command

```sh
cargo run --release
```

The `--release` indicates that an optimized release built should be run.
