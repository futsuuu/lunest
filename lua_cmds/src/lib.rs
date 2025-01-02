use std::{
    env::consts::{EXE_EXTENSION, EXE_SUFFIX},
    ffi::OsStr,
    path::Path,
};

#[derive(Debug, Clone, Copy)]
pub enum LuaCmd {
    #[cfg(feature = "lua51")]
    Lua51,
    #[cfg(feature = "lua52")]
    Lua52,
    #[cfg(feature = "lua53")]
    Lua53,
    #[cfg(feature = "lua54")]
    Lua54,
    #[cfg(feature = "luajit")]
    LuaJIT,
}

#[cfg(any(
    feature = "lua51",
    feature = "lua52",
    feature = "lua53",
    feature = "lua54",
    feature = "luajit",
))]
impl Default for LuaCmd {
    fn default() -> Self {
        #[cfg(feature = "lua54")]
        return LuaCmd::Lua54;
        #[cfg(all(feature = "lua53", not(feature = "lua54")))]
        return LuaCmd::Lua53;
        #[cfg(all(feature = "lua52", not(feature = "lua54"), not(feature = "lua53")))]
        return LuaCmd::Lua52;
        #[cfg(all(
            feature = "lua51",
            not(feature = "lua54"),
            not(feature = "lua53"),
            not(feature = "lua52"),
        ))]
        return LuaCmd::Lua51;
        #[cfg(all(
            feature = "luajit",
            not(feature = "lua54"),
            not(feature = "lua53"),
            not(feature = "lua52"),
            not(feature = "lua51"),
        ))]
        return LuaCmd::LuaJIT;
    }
}

impl LuaCmd {
    pub fn get_bytes(&self) -> &'static [u8] {
        use LuaCmd::*;
        match self {
            #[cfg(feature = "lua51")]
            Lua51 => include_bytes!(concat!(env!("OUT_DIR"), "/lua51")),
            #[cfg(feature = "lua52")]
            Lua52 => include_bytes!(concat!(env!("OUT_DIR"), "/lua52")),
            #[cfg(feature = "lua53")]
            Lua53 => include_bytes!(concat!(env!("OUT_DIR"), "/lua53")),
            #[cfg(feature = "lua54")]
            Lua54 => include_bytes!(concat!(env!("OUT_DIR"), "/lua54")),
            #[cfg(feature = "luajit")]
            LuaJIT => include_bytes!(concat!(env!("OUT_DIR"), "/luajit")),
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
            #[cfg(any(
                feature = "lua51",
                feature = "lua52",
                feature = "lua53",
                feature = "lua54",
                feature = "luajit",
            ))]
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
        });
        s.push_str(EXE_SUFFIX);
        s
    }
}
