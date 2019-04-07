A simple interpreter for Lua written in Rust.

## How to build

`cargo build --release`

## How to run the interpreter

`cargo run --release -p luavm -- filename.lua`

## How to run the compiler

`cargo run --release -p luacompiler -- filename.lua`

This will generate a `filename.luabc` file, which can also be used as input to
`luavm`.

## How to run the benchmarks on Debian GNU/Linux

You must have installed:
    * `rustc` and `cargo`
    * `make`
    * `python3`

The `./deps.sh` script installs `rustup` together with `rust`, `cargo`, and
other libraries that are needed by `lua` and `luajit`.

The `./build.sh` downloads, installs, and builds in this folder:
    * `multitime`
    * `lua5.3`
    * `luajit`
    * `luster`

To run a particular benchmark, such as `fib.lua`, you can use the `run.py`
script:

`./run.py -b ./benchmarks/fib.lua -n <number_of_times_to_run>`

For more options please see `./run.py --help`.
