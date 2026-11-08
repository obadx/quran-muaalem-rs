use boltffi::export;

/// Engine version. Throwaway export proving the BoltFFI toolchain end to end.
// TODO(#2): replace with the real engine API (A2) so this doesn't ship as the only export.
#[export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_crate() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
