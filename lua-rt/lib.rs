use std::{
    env::consts::{EXE_EXTENSION, EXE_SUFFIX},
    ffi::OsStr,
    path::Path,
};

fn decompress(data: &[u8], capacity: usize) -> Vec<u8> {
    let mut decoder = zstd::Decoder::new(data).unwrap();
    let mut buf = Vec::with_capacity(capacity);
    std::io::copy(&mut decoder, &mut buf).unwrap();
    buf
}
macro_rules! lazy_decompress {
    ($name:ident, $version:literal) => {
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

#[derive(Debug, Default, Clone, Copy)]
pub enum Lua {
    Lua51,
    Lua52,
    Lua53,
    #[default]
    Lua54,
}

impl Lua {
    pub fn get_bytes(&self) -> &'static [u8] {
        match self {
            Lua::Lua54 => LUA54.as_slice(),
            Lua::Lua53 => LUA53.as_slice(),
            Lua::Lua52 => LUA52.as_slice(),
            Lua::Lua51 => LUA51.as_slice(),
        }
    }

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
            "lua" => Some(Lua::default()),
            "lua5.1" => Some(Lua::Lua51),
            "lua5.2" => Some(Lua::Lua52),
            "lua5.3" => Some(Lua::Lua53),
            "lua5.4" => Some(Lua::Lua54),
            _ => None,
        }
    }

    pub fn recommended_program_name(&self) -> String {
        let mut s = String::with_capacity(10);
        s.push_str(match self {
            Lua::Lua51 => "lua5.1",
            Lua::Lua52 => "lua5.2",
            Lua::Lua53 => "lua5.3",
            Lua::Lua54 => "lua5.4",
        });
        s.push_str(EXE_SUFFIX);
        s
    }
}
