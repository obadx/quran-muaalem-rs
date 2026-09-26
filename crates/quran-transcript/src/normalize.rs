//! C2b-1: Imlaey text normalisation (`normalize_aya`).
//!
//! Ports `normalize_aya` from `utils.py`. Each filter is independent and
//! applied in the same order as Python: spaces, alef-maksora, hamazat,
//! taa-marboota, small-alef, tashkeel.

use crate::alphabet::imlaey;
use crate::aya::AyaError;

/// Options for [`normalize_aya`], mirroring the Python keyword arguments.
// The seven flags mirror Python `normalize_aya` one-to-one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct NormalizeOptions {
    /// Remove all whitespace.
    pub remove_spaces: bool,
    /// Map all hamazat (`ءآأؤإئ`) to hamza (`ء`).
    pub ignore_hamazat: bool,
    /// Map alef-maksora (`ى`) to alef (`ا`).
    pub ignore_alef_maksoora: bool,
    /// Map taa-marboota (`ة`) to haa (`ه`).
    pub ignore_taa_marboota: bool,
    /// Map taa-marboota (`ة`) to taa-mabsoota (`ت`).
    pub normalize_taat: bool,
    /// Remove the small alef (`ٰ`).
    pub remove_small_alef: bool,
    /// Remove tashkeel marks (`ًٌٍَُِّْ`).
    pub remove_tashkeel: bool,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            remove_spaces: true,
            ignore_hamazat: false,
            ignore_alef_maksoora: true,
            ignore_taa_marboota: false,
            normalize_taat: false,
            remove_small_alef: true,
            remove_tashkeel: false,
        }
    }
}

/// Removes every character of `class` from `text` (character-based).
fn remove_chars(text: &str, class: &str) -> String {
    text.chars().filter(|c| !class.contains(*c)).collect()
}

/// Maps every character of `from` in `text` to `to` (character-based).
fn map_chars(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if from.contains(c) {
            out.push_str(to);
        } else {
            out.push(c);
        }
    }
    out
}

/// Applies the selected filters to Imlaey `text`, in Python order.
///
/// # Errors
///
/// Returns [`AyaError::ConflictingTaaOptions`] when both
/// `ignore_taa_marboota` and `normalize_taat` are set.
pub fn normalize_aya(text: &str, options: &NormalizeOptions) -> Result<String, AyaError> {
    if options.ignore_taa_marboota && options.normalize_taat {
        return Err(AyaError::ConflictingTaaOptions);
    }

    let mut norm = text.to_owned();

    if options.remove_spaces {
        norm = norm.split_whitespace().collect();
    }

    if options.ignore_alef_maksoora {
        norm = norm.replace(imlaey::ALEF_MAKSOORA, imlaey::ALEF);
    }

    if options.ignore_hamazat {
        norm = map_chars(&norm, imlaey::HAMAZAT, imlaey::HAMZA);
    }

    if options.ignore_taa_marboota {
        norm = map_chars(&norm, imlaey::TAA_MARBOOTA, imlaey::HAA);
    }

    if options.normalize_taat {
        norm = map_chars(&norm, imlaey::TAA_MARBOOTA, imlaey::TAA_MABSOOTA);
    }

    if options.remove_small_alef {
        norm = norm.replace(imlaey::SMALL_ALEF, "");
    }

    if options.remove_tashkeel {
        norm = remove_chars(&norm, imlaey::TASHKEEL);
    }

    Ok(norm)
}

#[cfg(test)]
mod tests {
    use super::{NormalizeOptions, normalize_aya};
    use crate::aya::AyaError;

    fn opts() -> NormalizeOptions {
        NormalizeOptions::default()
    }

    #[test]
    fn defaults_remove_spaces_alef_maksoora_and_small_alef() {
        assert_eq!(normalize_aya("ا ب", &opts()), Ok("اب".to_owned()));
        assert_eq!(normalize_aya("ى", &opts()), Ok("ا".to_owned()));
        assert_eq!(normalize_aya("مـٰ", &opts()), Ok("مـ".to_owned()));
        // Tashkeel stays by default.
        assert_eq!(normalize_aya("بِسْمِ", &opts()), Ok("بِسْمِ".to_owned()));
    }

    #[test]
    fn each_filter_applies_in_isolation() {
        let off = NormalizeOptions {
            remove_spaces: false,
            ignore_hamazat: false,
            ignore_alef_maksoora: false,
            ignore_taa_marboota: false,
            normalize_taat: false,
            remove_small_alef: false,
            remove_tashkeel: false,
        };
        assert_eq!(normalize_aya("ا ب", &off), Ok("ا ب".to_owned()));

        assert_eq!(
            normalize_aya(
                "أإآؤئ",
                &NormalizeOptions {
                    ignore_hamazat: true,
                    ..off.clone()
                }
            ),
            Ok("ءءءءء".to_owned())
        );
        assert_eq!(
            normalize_aya(
                "ة",
                &NormalizeOptions {
                    ignore_taa_marboota: true,
                    ..off.clone()
                }
            ),
            Ok("ه".to_owned())
        );
        assert_eq!(
            normalize_aya(
                "ة",
                &NormalizeOptions {
                    normalize_taat: true,
                    ..off.clone()
                }
            ),
            Ok("ت".to_owned())
        );
        assert_eq!(
            normalize_aya(
                "بِسْمِ",
                &NormalizeOptions {
                    remove_tashkeel: true,
                    ..off
                }
            ),
            Ok("بسم".to_owned())
        );
    }

    #[test]
    fn conflicting_taa_options_are_rejected() {
        assert_eq!(
            normalize_aya(
                "ة",
                &NormalizeOptions {
                    ignore_taa_marboota: true,
                    normalize_taat: true,
                    ..opts()
                }
            ),
            Err(AyaError::ConflictingTaaOptions)
        );
    }

    #[test]
    fn combined_options_match_the_python_example() {
        // Mirrors the `normalize_aya` call sketched in `test_aya_obj.py`.
        let options = NormalizeOptions {
            remove_spaces: true,
            ignore_hamazat: false,
            ignore_alef_maksoora: false,
            ignore_taa_marboota: false,
            normalize_taat: false,
            remove_small_alef: true,
            remove_tashkeel: true,
        };
        assert_eq!(
            normalize_aya("الحمد لله", &options),
            Ok("الحمدلله".to_owned())
        );
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(normalize_aya("", &opts()), Ok(String::new()));
    }
}
