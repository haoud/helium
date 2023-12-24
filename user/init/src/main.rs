#![no_std]

extern crate alloc;

fn main() {
    // Get the framebuffer info from the kernel
    let framebuffer_info =
        iron::syscall::video::framebuffer_info().expect("Failed to get framebuffer info");

    // Print the framebuffer info to the serial port
    iron::syscall::serial::print(&alloc::format!(
        "Framebuffer info: {}x{}x{}\n",
        framebuffer_info.width,
        framebuffer_info.height,
        framebuffer_info.bpp
    ));
}
