# `lunest_lib`

This crate is compiled to a Lua C module, loaded from `./lua/lunest.lua`.

## Usage

```bash
lua ./lua/lunest.lua --help
```

## Testing

You must do followings before testing:

- Build this crate with the `test` feature
- Build the `lua_rt` crate with the debug profile

```bash
cargo build --package lunest_lib --features $LUA_FEATURE,test
cargo build --package lua_rt --features $LUA_FEATURE
cargo test --package lunest_lib --features $LUA_FEATURE
```
