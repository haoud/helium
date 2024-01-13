#[macro_export]
#[allow_internal_unstable(print_internals, format_args_nl)]
macro_rules! println {
    () => {
        $crate::::syscall::serial::print("\n")
    };
    ($($arg:tt)*) => {{
        $crate::syscall::serial::print(&alloc::format!($($arg)*));
        $crate::syscall::serial::print("\n");
    }};
}