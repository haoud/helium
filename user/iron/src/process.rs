/// This type represents the status code the current process can return to its
/// parent under normal termination.
pub struct ExitCode(i32);

impl ExitCode {
    pub const SUCCESS: ExitCode = ExitCode(0);
    pub const FAILURE: ExitCode = ExitCode(1);

    pub const fn to_i32(self) -> i32 {
        self.0
    }
}

impl From<u8> for ExitCode {
    fn from(val: u8) -> ExitCode {
        ExitCode(val as i32)
    }
}

/// This trait is implemented by types that can be returned by the `main` function of the
/// application. The `main` function can return any type that implements this trait and the
/// runtime will convert it to an ExitCode that will be handled by the runtime and returned
/// to the parent process.
pub trait Termination {
    fn report(self) -> ExitCode;
}

impl Termination for ! {
    fn report(self) -> ExitCode {
        self
    }
}

impl Termination for () {
    fn report(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

impl Termination for ExitCode {
    fn report(self) -> ExitCode {
        self
    }
}

impl<T: Termination, E: core::fmt::Debug> Termination for Result<T, E> {
    fn report(self) -> ExitCode {
        match self {
            Ok(val) => val.report(),
            Err(_) => {
                // TODO: Print an error message
                ExitCode::FAILURE
            }
        }
    }
}
