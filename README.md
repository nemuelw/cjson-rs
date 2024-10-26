# cjson-rs

Rust bindings for the cJSON library

## Setup

Make sure you have the [cJSON](https://github.com/DaveGamble/cJSON) library properly installed your system.

- Set the `CJSON_INCLUDE_PATH` and `CJSON_LIB_PATH` environment variables:
  - `CJSON_INCLUDE_PATH` specifies where the compiler should look for the `cJSON.h` header file during the compilation process eg. `/usr/include/cjson`.
  - `CJSON_LIB_PATH` specifies where the linker should look for precompiled cJSON library files eg. `/usr/local/lib`.

- Update your `Cargo.toml` file by adding this to the `[dependencies]` section:

    ```toml
    cjson-rs = "0.2.3"
    ```

- Import the crate in your source file(s) to start using it:

    ```rust
    use cjson_rs::*;
    ```

## Documentation

For detailed documentation, please refer to the documentation comments in the code. You can view it even
better in your browser by running this command in the `cjson-rs` project directory:

```bash
cargo doc --open
```

## Contributing

Contributions are much welcome! Feel free to open issues or create pull requests to contribute to this
project.

## License

This project is licensed under the [MIT License](./LICENSE).
