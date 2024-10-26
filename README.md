# cjson-rs

Rust bindings for the cJSON library

## Setup

Make sure you have the [cJSON](https://github.com/DaveGamble/cJSON) library properly installed your system.

- Set the `CJSON_INCLUDE_PATH` and `CJSON_LIB_PATH` environment variables:
  - `CJSON_INCLUDE_PATH` specifies where the compiler should look for the `cJSON.h` header file during the compilation process eg. `/usr/include/cjson`.
  - `CJSON_LIB_PATH` specifies where the linker should look for precompiled cJSON library files eg. `/usr/local/lib`.

- Update your `Cargo.toml` file:

    ```toml
    cjson-rs = "0.1.0"
    ```

- Import the crate to start using it:

    ```rust
    use cjson_rs::*;
    ```
