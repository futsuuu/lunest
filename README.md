# Lunest

Lunest is a testing framework for Lua 5.1 ~ 5.4, built on [Rust](https://www.rust-lang.org) and [mlua](https://crates.io/crates/mlua).

```lua
local lunest = require('lunest')

lunest.group('separated environment', function()
    local one = 1

    lunest.test('one equals 1', function()
        assert(one == 1)
        one = 2
    end)

    lunest.test('one still equals 1', function()
        assert(one == 1)
        one = 2
    end)
end)
```

## Features

- **Cross-platform**: works on Linux, macOS and Windows
- **Easy to install**: works with a single binary, so you don't need to install Lua and LuaRocks
- **Inline testing**: for private/small functions

## Requirements

- A command that can run Lua files (e.g. `lua`, `nvim -l`)

## Installation

TODO

## Command-line API

### `lunset run`

Run tests.

#### Arguments

none

## Lua API

### `require("lunest").test(name, func)`

Define a test.

#### Arguments

- **name** (_string_): test name
- **func** (_function_): test function

### `require("lunest").group(name, func)`

This function should be only used to group some related tests or groups.

#### Arguments

- **name** (_string_): group name
- **func** (_function_): defining child groups and tests

## Configuration

You can use a TOML file located in `.lunest/config.toml` in current directory for configuration.

### Example

```toml
# .lunest/config.toml

[profile.default]
lua = ["lua"]
files = [
    "tests/**/*.lua",
]
setup = ".lunest/setup.lua"

[profile.all]
files = [
    "**/*.lua",
    "!scripts/",
]
```

```lua
-- .lunest/setup.lua

_G._TEST = true
```

## Building from source

### Requirements

- Rust

### Download source code

```bash
git clone https://github.com/futsuuu/lunest.git
cd lunest
```

### Build / Install

Run following to build:

```bash
cargo xtask --release build  # ./target/release/lunest
```

Or install via `cargo install`:

```bash
cargo xtask --release install  # ~/.cargo/bin/lunest
```

## License

This repository is licensed under the [MIT license](./LICENSE).
