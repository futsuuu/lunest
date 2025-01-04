use std::{
    env::consts::{EXE_EXTENSION, EXE_SUFFIX},
    ffi::OsStr,
    path::Path,
};

#[cfg(zstd_dict)]
static ZSTD_DICT: std::sync::LazyLock<zstd::dict::DecoderDictionary<'static>> =
    std::sync::LazyLock::new(|| {
        zstd::dict::DecoderDictionary::copy(include_bytes!(concat!(env!("OUT_DIR"), "/zstd_dict")))
    });
#[cfg(zstd_dict)]
fn decompress(data: &[u8], capacity: usize) -> Vec<u8> {
    let dict = &*ZSTD_DICT;
    let mut decoder = zstd::Decoder::with_prepared_dictionary(data, dict).unwrap();
    let mut buf = Vec::with_capacity(capacity);
    std::io::copy(&mut decoder, &mut buf).unwrap();
    buf
}
#[cfg(not(zstd_dict))]
fn decompress(data: &[u8], capacity: usize) -> Vec<u8> {
    let mut decoder = zstd::Decoder::new(data).unwrap();
    let mut buf = Vec::with_capacity(capacity);
    std::io::copy(&mut decoder, &mut buf).unwrap();
    buf
}

macro_rules! lazy_decompress {
    ($name:ident, $version:literal) => {
        #[cfg(feature = $version)]
        static $name: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
            decompress(
                include_bytes!(concat!(env!("OUT_DIR"), "/", $version, ".zst")),
                include!(concat!(env!("OUT_DIR"), "/", $version, "_size.rs")),
            )
        });
    };
}
lazy_decompress!(LUA54, "lua54");
lazy_decompress!(LUA53, "lua53");
lazy_decompress!(LUA52, "lua52");
lazy_decompress!(LUA51, "lua51");
lazy_decompress!(LUAJIT, "luajit");

#[derive(Debug, Default, Clone, Copy)]
pub enum LuaCmd {
    #[cfg(feature = "lua51")]
    #[cfg_attr(default_lua = "lua51", default)]
    Lua51,
    #[cfg(feature = "lua52")]
    #[cfg_attr(default_lua = "lua52", default)]
    Lua52,
    #[cfg(feature = "lua53")]
    #[cfg_attr(default_lua = "lua53", default)]
    Lua53,
    #[cfg(feature = "lua54")]
    #[cfg_attr(default_lua = "lua54", default)]
    Lua54,
    #[cfg(feature = "luajit")]
    #[cfg_attr(default_lua = "luajit", default)]
    LuaJIT,
    #[cfg(default_lua = "none")]
    #[default]
    None,
}

impl LuaCmd {
    #[cfg(not(default_lua = "none"))]
    pub fn get_bytes(&self) -> &'static [u8] {
        use LuaCmd::*;
        match self {
            #[cfg(feature = "lua54")]
            Lua54 => LUA54.as_slice(),
            #[cfg(feature = "lua53")]
            Lua53 => LUA53.as_slice(),
            #[cfg(feature = "lua52")]
            Lua52 => LUA52.as_slice(),
            #[cfg(feature = "lua51")]
            Lua51 => LUA51.as_slice(),
            #[cfg(feature = "luajit")]
            LuaJIT => LUAJIT.as_slice(),
        }
    }

    #[cfg(not(default_lua = "none"))]
    pub fn write(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(&path, self.get_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }

    pub fn from_program_name(program: impl AsRef<OsStr>) -> Option<Self> {
        let program = Path::new(program.as_ref());
        let file_name = if program.extension() == Some(OsStr::new(EXE_EXTENSION)) {
            program.file_stem()?
        } else {
            program.file_name()?
        };
        match file_name.to_str()? {
            "lua" => Some(Self::default()),
            #[cfg(feature = "lua51")]
            "lua5.1" => Some(LuaCmd::Lua51),
            #[cfg(feature = "lua52")]
            "lua5.2" => Some(LuaCmd::Lua52),
            #[cfg(feature = "lua53")]
            "lua5.3" => Some(LuaCmd::Lua53),
            #[cfg(feature = "lua54")]
            "lua5.4" => Some(LuaCmd::Lua54),
            #[cfg(feature = "luajit")]
            "luajit" => Some(LuaCmd::LuaJIT),
            _ => None,
        }
    }

    pub fn recommended_program_name(&self) -> String {
        let mut s = String::with_capacity(10);
        use LuaCmd::*;
        s.push_str(match self {
            #[cfg(feature = "lua51")]
            Lua51 => "lua5.1",
            #[cfg(feature = "lua52")]
            Lua52 => "lua5.2",
            #[cfg(feature = "lua53")]
            Lua53 => "lua5.3",
            #[cfg(feature = "lua54")]
            Lua54 => "lua5.4",
            #[cfg(feature = "luajit")]
            LuaJIT => "luajit",
            #[allow(unreachable_patterns)]
            _ => "lua",
        });
        s.push_str(EXE_SUFFIX);
        s
    }
}
