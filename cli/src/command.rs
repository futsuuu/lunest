#[derive(Debug, Clone, PartialEq)]
pub struct Builder {
    program: std::ffi::OsString,
    args: Vec<std::ffi::OsString>,
    env: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    cwd: Option<std::path::PathBuf>,
}

impl Builder {
    pub fn new(program: impl Into<std::ffi::OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            cwd: None,
        }
    }

    pub fn program(&mut self, program: impl Into<std::ffi::OsString>) -> &mut Self {
        self.program = program.into();
        self
    }

    pub fn get_program(&self) -> &std::ffi::OsStr {
        &self.program
    }

    pub fn arg(&mut self, arg: impl Into<std::ffi::OsString>) -> &mut Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(
        &mut self,
        args: impl IntoIterator<Item = impl Into<std::ffi::OsString>>,
    ) -> &mut Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(
        &mut self,
        key: impl Into<std::ffi::OsString>,
        val: impl Into<std::ffi::OsString>,
    ) -> &mut Self {
        self.env.insert(key.into(), val.into());
        self
    }

    pub fn current_dir(&mut self, dir: impl Into<std::path::PathBuf>) -> &mut Self {
        self.cwd.replace(dir.into());
        self
    }

    pub fn build(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.program);
        cmd.args(self.args.iter());
        cmd.envs(self.env.iter());
        if let Some(dir) = &self.cwd {
            cmd.current_dir(dir);
        }
        cmd
    }

    pub fn display(&self) -> Display<'_> {
        Display::from(self)
    }
}

impl<T: Into<std::ffi::OsString>> From<T> for Builder {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

pub struct Display<'a> {
    builder: &'a Builder,
    env: bool,
}

impl<'a> From<&'a Builder> for Display<'a> {
    fn from(builder: &'a Builder) -> Self {
        Display {
            builder,
            env: false,
        }
    }
}

impl<'a> Display<'a> {
    pub fn env(&'a mut self, enable: bool) -> &'a mut Self {
        self.env = enable;
        self
    }
}

impl std::fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.env {
            for (key, val) in &self.builder.env {
                write!(f, "{}='{}' ", key.to_string_lossy(), val.to_string_lossy())?;
            }
        }
        write!(f, "{}", self.builder.program.to_string_lossy())?;
        for arg in &self.builder.args {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}
