use smithay::{output::Output, reexports::wayland_server::protocol::wl_surface::WlSurface};

pub mod winit;

/// A trait defining common methods for each available backend: winit and tty-udev
pub trait Backend: 'static {
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);

    // INFO: only for udev in anvil, maybe shouldn't be a trait fn?
    fn early_import(&mut self, surface: &WlSurface);
}