use std::sync::LazyLock;

use serde::Deserialize;

/// The top-level shape of the embedded Quran text file.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LoadedQuranDocument {
    pub quran: LoadedQuran,
}

/// All suras in Quran order.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LoadedQuran {
    #[serde(rename = "sura")]
    pub surahs: Vec<LoadedSura>,
}

/// A sura and its ayat.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LoadedSura {
    #[serde(rename = "@index")]
    pub index: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "aya")]
    pub ayahs: Vec<LoadedAya>,
}

/// One ayah in both Uthmani and Imlaey scripts.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LoadedAya {
    #[serde(rename = "@index")]
    pub index: String,
    #[serde(rename = "@uthmani")]
    pub uthmani: String,
    #[serde(rename = "@imlaey")]
    pub imlaey: String,
    /// Bismillah in Uthmani script (first aya of every sura except 1 and 9).
    #[serde(rename = "@bismillah_uthmani", default)]
    pub bismillah_uthmani: Option<String>,
    /// Bismillah in Imlaey script (first aya of every sura except 1 and 9).
    #[serde(rename = "@bismillah_imlaey", default)]
    pub bismillah_imlaey: Option<String>,
}

static QURAN_TEXT: LazyLock<LoadedQuranDocument> = LazyLock::new(|| {
    serde_json::from_str(crate::QURAN_UTHMANI_IMLAEY_JSON)
        .expect("embedded Uthmani-Imlaey Quran JSON must remain valid")
});

/// Returns the parsed Quran text containing Uthmani and Imlaey ayat.
///
/// The JSON is parsed only once and shared for the lifetime of the program.
#[must_use]
pub fn quran_text() -> &'static LoadedQuranDocument {
    &QURAN_TEXT
}

#[cfg(test)]
mod tests {
    use super::quran_text;

    #[test]
    fn quran_text_has_the_expected_structure() {
        let document = quran_text();

        assert_eq!(document.quran.surahs.len(), 114);

        let fatiha = &document.quran.surahs[0];
        assert_eq!(fatiha.index, "1");
        assert_eq!(fatiha.name, "الفاتحة");
        assert_eq!(fatiha.ayahs.len(), 7);

        let first_ayah = &fatiha.ayahs[0];
        assert_eq!(first_ayah.index, "1");
        assert!(!first_ayah.uthmani.is_empty());
        assert!(!first_ayah.imlaey.is_empty());
    }
}
