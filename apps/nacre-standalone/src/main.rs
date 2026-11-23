//! Nacre standalone application entry point.

fn main() {
    println!("nacre {}", env!("CARGO_PKG_VERSION"));
    println!("audio: no inicializado");
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_metadata_is_present() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nacre-standalone");
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }
}
