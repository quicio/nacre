//! Ports through which application logic accesses external capabilities.

use std::time::Duration;

/// Controls the lifetime of an audio host.
pub trait AudioHost {
    /// Error reported by the host implementation.
    type Error;

    /// Starts audio processing.
    fn start(&mut self) -> Result<(), Self::Error>;

    /// Stops audio processing.
    fn stop(&mut self) -> Result<(), Self::Error>;
}

/// Provides a monotonic time source.
pub trait Clock {
    /// Returns the elapsed monotonic time since an implementation-defined origin.
    fn now(&self) -> Duration;
}

/// Renders audio into a caller-owned sample buffer.
pub trait Renderer {
    /// Error reported by the renderer implementation.
    type Error;

    /// Fills `output` with rendered samples.
    fn render(&mut self, output: &mut [f32]) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::{AudioHost, Clock, Renderer};

    fn assert_object_safe<T: ?Sized>() {}

    #[test]
    fn ports_are_object_safe() {
        assert_object_safe::<dyn AudioHost<Error = Infallible>>();
        assert_object_safe::<dyn Clock>();
        assert_object_safe::<dyn Renderer<Error = Infallible>>();
    }
}
