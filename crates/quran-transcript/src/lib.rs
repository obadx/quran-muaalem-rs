pub mod alphabet;
pub mod aya;
pub mod normalize;
pub mod quran_data;
pub mod search;

/// The original Quran alphabet data, embedded into this crate at build time.
pub const QURAN_ALPHABET_JSON: &str = include_str!("../assets/quran-alphabet.json");

/// Words used to determine how an initial hamzat wasl is pronounced.
pub const BEGIN_WITH_HAMZAT_WASL_JSON: &str = include_str!("../assets/begin_with_hamzat_wasl.json");

/// The Uthmani-to-Imlaey Quran text data, embedded at build time.
pub const QURAN_UTHMANI_IMLAEY_JSON: &str = include_str!("../assets/quran-uthmani-imlaey.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_quran_assets_are_embedded_and_valid_json() {
        let alphabet: serde_json::Value =
            serde_json::from_str(QURAN_ALPHABET_JSON).expect("alphabet JSON must be valid");

        let hamzat_wasl: serde_json::Value = serde_json::from_str(BEGIN_WITH_HAMZAT_WASL_JSON)
            .expect("hamzat-wasl JSON must be valid");

        let uthmani_imlaey: serde_json::Value = serde_json::from_str(QURAN_UTHMANI_IMLAEY_JSON)
            .expect("Uthmani-Imlaey JSON must be valid");

        assert!(alphabet.is_object());
        assert!(hamzat_wasl.is_object());
        assert!(uthmani_imlaey.is_object());
    }
}
