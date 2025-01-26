pub struct Context {
    config: crate::config::Config,

    temp_dir: tempfile::TempDir,
    main_script: std::path::PathBuf,
    lua_programs: std::cell::RefCell<
        std::collections::HashMap<std::rc::Rc<std::ffi::OsString>, std::rc::Rc<std::ffi::OsString>>,
    >,
    process_dir_counter: std::cell::Cell<usize>,
}

#[derive(clap::Args, Debug)]
pub struct ContextOptions {
    /// Don't clean up a temporary directory on exit
    #[arg(long)]
    keep_tmpdir: bool,
}

impl Context {
    pub fn new(opts: &ContextOptions) -> anyhow::Result<Self> {
        log::trace!("creating new global context");
        let config = crate::config::Config::read()?;
        let temp_dir = tempfile::Builder::new()
            .prefix(env!("CARGO_PKG_NAME"))
            .keep(opts.keep_tmpdir)
            .tempdir()?;
        let main_script = temp_dir.path().join("main.lua");
        std::fs::write(
            &main_script,
            include_str!(concat!(env!("OUT_DIR"), "/main.lua")),
        )?;
        Ok(Self {
            config,
            temp_dir,
            main_script,
            lua_programs: std::cell::RefCell::new(std::collections::HashMap::new()),
            process_dir_counter: std::cell::Cell::new(0),
        })
    }

    pub fn root_dir(&self) -> &std::path::Path {
        self.config.root_dir()
    }

    pub fn config(&self) -> &crate::config::Config {
        &self.config
    }

    pub fn create_process_dir(&self) -> std::io::Result<std::path::PathBuf> {
        let counter = self.process_dir_counter.get();
        let name = format!("p{:x}", counter);
        self.process_dir_counter.set(counter + 1);
        let dir = self.temp_dir.path().join(name);
        std::fs::create_dir(&dir)?;
        Ok(dir)
    }

    pub fn get_main_script(&self) -> &std::path::Path {
        &self.main_script
    }

    pub fn get_lua_program(
        &self,
        name: impl Into<std::ffi::OsString>,
    ) -> std::io::Result<impl std::ops::Deref<Target = std::ffi::OsString>> {
        let name = std::rc::Rc::new(name.into());
        if let Some(program) = self.lua_programs.borrow().get(&name) {
            return Ok(std::rc::Rc::clone(program));
        }
        let program = if let Ok(path) = which::which(&*name) {
            std::rc::Rc::new(path.into())
        } else if let Some(lua) = lua_rt::Lua::from_program_name(&*name) {
            let path = self.temp_dir.path().join(lua.recommended_program_name());
            lua.write(&path)?;
            std::rc::Rc::new(path.into())
        } else {
            std::rc::Rc::clone(&name)
        };
        self.lua_programs
            .borrow_mut()
            .insert(name, std::rc::Rc::clone(&program));
        Ok(program)
    }
}
