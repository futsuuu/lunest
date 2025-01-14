pub struct RuntimeFiles {
    inner: tempfile::TempDir,
    main_script: std::path::PathBuf,
    lua_programs: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    process_dir_counter: usize,
}

impl RuntimeFiles {
    pub fn new() -> std::io::Result<Self> {
        let temp_dir = tempfile::TempDir::with_prefix(env!("CARGO_PKG_NAME"))?;
        let main_script = temp_dir.path().join("main.lua");
        std::fs::write(
            &main_script,
            include_str!(concat!(env!("OUT_DIR"), "/main.lua")),
        )?;
        Ok(Self {
            inner: temp_dir,
            main_script,
            lua_programs: std::collections::HashMap::new(),
            process_dir_counter: 0,
        })
    }

    pub fn create_process_dir(&mut self) -> std::io::Result<std::path::PathBuf> {
        let name = format!("p{:x}", self.process_dir_counter);
        self.process_dir_counter += 1;
        let dir = self.inner.path().join(name);
        std::fs::create_dir(&dir)?;
        Ok(dir)
    }

    pub fn get_main_script(&self) -> &std::path::Path {
        &self.main_script
    }

    pub fn get_lua_program(
        &mut self,
        name: impl Into<std::ffi::OsString>,
    ) -> std::io::Result<std::ffi::OsString> {
        let name = name.into();
        if let Some(program) = self.lua_programs.get(&name) {
            return Ok(program.clone());
        }
        let program = if let Ok(path) = which::which(&name) {
            path.into_os_string()
        } else if let Some(lua) = lua_rt::Lua::from_program_name(&name) {
            let path = self.inner.path().join(lua.recommended_program_name());
            lua.write(&path)?;
            path.into_os_string()
        } else {
            name.clone()
        };
        self.lua_programs.insert(name.clone(), program.clone());
        Ok(program)
    }
}
