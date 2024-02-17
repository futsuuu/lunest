# Lunest

A tesing framework for Lua

```lua
local lunest = require('lunest')

lunest.group('calculation', function()
  lunest.test('one plus one equals two', function()
    assert(1 + 1 == 2)
  end)
end)
```

## Features

- **Cross-platform**: works on Linux, macOS and Windows
- **Easy to install**: compiled to single binary
- **Inline tests**: testing private/small functions

## Installation

### Building from source

```bash
git clone https://github.com/futsuuu/lunest.git && cd lunest
cargo xtask --release install
lunest --help
```

## License

This repository is licensed under the [MIT license](./LICENSE).
