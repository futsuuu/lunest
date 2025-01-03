use std::{
    env::consts::{EXE_EXTENSION, EXE_SUFFIX},
    ffi::OsStr,
    path::Path,
};

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
