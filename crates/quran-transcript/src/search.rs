//! C2b-2: Imlaey search over the Quran.
//!
//! Ports `search`, `SearchItem`, `_get_words_span`,
//! `_get_uthmani_of_result_item` and `_get_imlaey_words_and_str` from
//! `utils.py`.
//!
//! Matching is literal substring search on spaceless text
//! (`str::match_indices`, same semantics as Python's `finditer` over an
//! escaped pattern). All match offsets are converted from bytes to
//! character indices before word-span mapping, so Arabic text is never
//! byte-indexed.

use std::fmt;

use crate::{
    aya::{Aya, AyaError, EncodeFlags, WordSpan},
    normalize::{NormalizeOptions, normalize_aya},
};

/// One search hit, mirroring Python `SearchItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchItem {
    /// Aya where the match starts (`None` for istiaatha-only queries).
    pub start_aya: Option<Aya>,
    /// Number of ayat the match spans.
    pub num_ayat: usize,
    /// Word span inside the start (and end) aya.
    pub imlaey_word_span: Option<WordSpan>,
    /// Uthmani script of the match.
    pub uthmani_script: String,
    /// Whether Bismillah words were included in the match.
    pub has_bismillah: bool,
    /// Whether the query carried istiaatha.
    pub has_istiaatha: bool,
}

impl fmt::Display for SearchItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.start_aya {
            Some(aya) => write!(
                f,
                "start_aya(sura_idx={}, aya_idx={})",
                aya.sura_idx(),
                aya.aya_idx()
            ),
            None => write!(f, "start_aya(sura_idx=None, aya_idx=None)"),
        }?;
        write!(
            f,
            ", num_ayat={}, uthmani_script={}, has_istiaatha={}, has_bismillah={}, imlaey_word_span={:?}",
            self.num_ayat,
            self.uthmani_script,
            self.has_istiaatha,
            self.has_bismillah,
            self.imlaey_word_span,
        )
    }
}

/// Options for [`search`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    /// Ayat around `start_aya`: `[start - window/2, start + window/2]`.
    pub window: usize,
    /// Normalisation applied to query and corpus.
    pub normalize: NormalizeOptions,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            window: 2,
            normalize: NormalizeOptions::default(),
        }
    }
}

/// Position of a word inside the search blob: aya offset plus word offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vertex {
    aya_idx: usize,
    word_idx: usize,
}

/// Searches the Holy Quran for Imlaey `text` around `start_aya`.
///
/// # Errors
///
/// Returns [`AyaError`] for invalid normalisation options, or when a match
/// cuts inside a merged Uthmani word (Python raises `PartOfUthmaniWord`
/// out of `search` in that case).
pub fn search(
    text: &str,
    start_aya: &Aya,
    options: &SearchOptions,
) -> Result<Vec<SearchItem>, AyaError> {
    let mut query_opts = options.normalize.clone();
    query_opts.remove_spaces = true;
    let mut normalized = normalize_aya(text, &query_opts)?;
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let half = crate::aya::i32_len(options.window / 2);
    let loop_aya = start_aya.step(-half);

    // Strip a leading istiaatha from the query, if present.
    let mut corpus_opts = options.normalize.clone();
    corpus_opts.remove_spaces = false;
    let istiaatha_blob: String = normalize_aya(&start_aya.get().istiaatha_imlaey, &corpus_opts)?
        .split(' ')
        .collect();
    let mut has_istiaatha = false;
    if !istiaatha_blob.is_empty()
        && let Some(pos) = normalized.find(&istiaatha_blob)
    {
        normalized = normalized[pos + istiaatha_blob.len()..].to_owned();
        has_istiaatha = true;
        if normalized.is_empty() {
            return Ok(vec![SearchItem {
                start_aya: None,
                num_ayat: 0,
                imlaey_word_span: None,
                has_bismillah: false,
                has_istiaatha,
                uthmani_script: start_aya.get().istiaatha_uthmani,
            }]);
        }
    }

    for include_bismillah in [false, true] {
        let (words, blob) =
            imlaey_words_and_str(&loop_aya, options.window, include_bismillah, options)?;
        let query_chars = normalized.chars().count();
        let mut found = Vec::new();
        for (byte_start, _) in blob.match_indices(normalized.as_str()) {
            let char_start = blob[..byte_start].chars().count();
            let char_end = char_start + query_chars;
            if let Some((start_v, end_v)) = get_words_span(char_start, char_end, &words) {
                let span = WordSpan {
                    start: start_v.word_idx,
                    end: Some(end_v.word_idx),
                };
                let match_aya = loop_aya.step(crate::aya::i32_len(start_v.aya_idx));
                let item = SearchItem {
                    uthmani_script: uthmani_of_result_item(
                        &match_aya,
                        end_v.aya_idx - start_v.aya_idx + 1,
                        span,
                        include_bismillah,
                    )?,
                    start_aya: Some(match_aya),
                    num_ayat: end_v.aya_idx - start_v.aya_idx + 1,
                    imlaey_word_span: Some(span),
                    has_bismillah: include_bismillah,
                    has_istiaatha,
                };
                found.push(item);
            }
        }
        if !found.is_empty() {
            if has_istiaatha {
                let prefix = start_aya.get().istiaatha_uthmani;
                for item in &mut found {
                    item.uthmani_script = format!("{prefix} {}", item.uthmani_script);
                }
            }
            return Ok(found);
        }
    }

    Ok(Vec::new())
}

/// Maps a character span of the spaceless blob to word positions.
///
/// Only word-boundary matches are accepted: the start must sit exactly at a
/// word start and the end exactly past a word end, else `None` (this is what
/// rejects mid-word matches like `"حم"` inside `"الحمد"`).
fn get_words_span(start: usize, end: usize, words: &[Vec<String>]) -> Option<(Vertex, Vertex)> {
    let mut count = 0;
    let mut start_v = None;
    for (aya_idx, aya) in words.iter().enumerate() {
        for (word_idx, word) in aya.iter().enumerate() {
            if start == count {
                start_v = Some(Vertex { aya_idx, word_idx });
                break;
            }
            count += word.chars().count();
        }
        if start_v.is_some() {
            break;
        }
    }
    let start_v = start_v?;

    let mut count = start;
    for (offset, aya) in words[start_v.aya_idx..].iter().enumerate() {
        let aya_idx = start_v.aya_idx + offset;
        let skip = if offset == 0 { start_v.word_idx } else { 0 };
        for (word_idx, word) in aya.iter().enumerate().skip(skip) {
            count += word.chars().count();
            if end == count {
                return Some((
                    start_v,
                    Vertex {
                        aya_idx,
                        word_idx: word_idx + 1,
                    },
                ));
            }
        }
    }
    None
}

/// Rebuilds the Uthmani script of a search hit across its ayat.
fn uthmani_of_result_item(
    start_aya: &Aya,
    num_ayat: usize,
    span: WordSpan,
    include_bismillah: bool,
) -> Result<String, AyaError> {
    let flags = EncodeFlags {
        bismillah: include_bismillah,
        ..EncodeFlags::default()
    };
    let ayat: Vec<Aya> = start_aya.ayat_after(Some(num_ayat)).collect();
    let mut out = String::new();
    for (idx, aya) in ayat.iter().enumerate() {
        let word_span = WordSpan {
            start: if idx == 0 { span.start } else { 0 },
            end: if idx + 1 == ayat.len() {
                span.end
            } else {
                None
            },
        };
        out.push_str(&aya.imlaey_to_uthmani(word_span, flags)?);
        out.push(' ');
    }
    if out.ends_with(' ') {
        out.pop();
    }
    Ok(out)
}

/// Collects normalised Imlaey words plus their spaceless blob.
fn imlaey_words_and_str(
    start_aya: &Aya,
    window: usize,
    include_bismillah: bool,
    options: &SearchOptions,
) -> Result<(Vec<Vec<String>>, String), AyaError> {
    let mut word_opts = options.normalize.clone();
    word_opts.remove_spaces = false;
    let mut words_all = Vec::new();
    let mut blob = String::new();
    for aya in start_aya.ayat_after(Some(window.saturating_add(1))) {
        let view = aya.get();
        let mut aya_words = Vec::new();
        if include_bismillah && let Some(bismillah) = view.bismillah_imlaey {
            aya_words.extend(
                normalize_aya(&bismillah, &word_opts)?
                    .split(' ')
                    .map(str::to_owned),
            );
        }
        aya_words.extend(
            normalize_aya(&view.imlaey, &word_opts)?
                .split(' ')
                .map(str::to_owned),
        );
        blob.extend(aya_words.iter().flat_map(|word| word.chars()));
        words_all.push(aya_words);
    }
    Ok((words_all, blob))
}

#[cfg(test)]
// Expected strings are byte-exact Quranic corpus text, which is not
// NFC-normalised (combining-mark order is significant here).
#[allow(clippy::unicode_not_nfc)]
mod tests {
    use super::{SearchOptions, search};
    use crate::{aya::Aya, normalize::NormalizeOptions};

    fn tashkeel_opts(window: usize) -> SearchOptions {
        SearchOptions {
            window,
            normalize: NormalizeOptions {
                remove_tashkeel: true,
                ..NormalizeOptions::default()
            },
        }
    }

    #[test]
    fn finds_a_hit_with_word_span_and_uthmani() {
        let hits = search(
            "الحمد لله",
            &Aya::new(1, 1).expect("1:1"),
            &tashkeel_opts(4),
        )
        .expect("search must succeed");

        assert_eq!(hits.len(), 1);
        let hit = &hits[0];
        assert_eq!(hit.start_aya.expect("hit has a start").get().sura_idx, 1);
        assert_eq!(hit.start_aya.expect("hit has a start").get().aya_idx, 2);
        assert_eq!(hit.num_ayat, 1);
        assert!(hit.imlaey_word_span.is_some());
        assert_eq!(hit.uthmani_script, "ٱلْحَمْدُ لِلَّهِ");
        assert!(!hit.has_istiaatha);
        assert!(!hit.has_bismillah);
    }

    #[test]
    fn finds_a_full_aya_further_from_the_pivot() {
        let hits = search(
            "إياك نعبد وإياك نستعين",
            &Aya::new(1, 1).expect("1:1"),
            &tashkeel_opts(10),
        )
        .expect("search must succeed");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].start_aya.expect("hit has a start").get().aya_idx, 5);
        assert_eq!(hits[0].uthmani_script, "إِيَّاكَ نَعْبُدُ وَإِيَّاكَ نَسْتَعِينُ");
    }

    #[test]
    fn rejects_mid_word_matches() {
        let hits = search("حم", &Aya::new(1, 1).expect("1:1"), &tashkeel_opts(4))
            .expect("search must succeed");
        assert!(hits.is_empty());
    }

    #[test]
    fn empty_query_yields_no_hits() {
        let hits = search("", &Aya::new(1, 1).expect("1:1"), &tashkeel_opts(4))
            .expect("search must succeed");
        assert!(hits.is_empty());
    }

    #[test]
    fn default_options_keep_tashkeel_and_miss_bare_queries() {
        // Verified against Python: the bare query cannot match the
        // fully-vocalised corpus without `remove_tashkeel`.
        let hits = search(
            "الحمد لله",
            &Aya::new(1, 1).expect("1:1"),
            &SearchOptions::default(),
        )
        .expect("search must succeed");
        assert!(hits.is_empty());
    }

    #[test]
    fn istiaatha_only_query_returns_a_marker_hit() {
        let hits = search(
            "أعوذ بالله من الشيطان الرجيم",
            &Aya::new(1, 1).expect("1:1"),
            &tashkeel_opts(4),
        )
        .expect("search must succeed");

        assert_eq!(hits.len(), 1);
        assert!(hits[0].start_aya.is_none());
        assert_eq!(hits[0].num_ayat, 0);
        assert!(hits[0].imlaey_word_span.is_none());
        assert!(hits[0].has_istiaatha);
        assert_eq!(hits[0].uthmani_script, "أَعُوذُ بِٱللَّهِ مِنَ ٱلشَّيْطَانِ ٱلرَّجِيمِ");
    }

    #[test]
    fn istiaatha_prefixed_query_marks_and_prefixes_hits() {
        let hits = search(
            "أعوذ بالله من الشيطان الرجيم الحمد لله",
            &Aya::new(1, 1).expect("1:1"),
            &tashkeel_opts(4),
        )
        .expect("search must succeed");

        assert_eq!(hits.len(), 1);
        assert!(hits[0].has_istiaatha);
        assert!(
            hits[0]
                .uthmani_script
                .starts_with("أَعُوذُ بِٱللَّهِ مِنَ ٱلشَّيْطَانِ ٱلرَّجِيمِ ")
        );
    }

    #[test]
    fn bismillah_query_uses_the_second_pass() {
        let hits = search(
            "بسم الله الرحمن الرحيم",
            &Aya::new(2, 1).expect("2:1"),
            &tashkeel_opts(4),
        )
        .expect("search must succeed");

        assert_eq!(hits.len(), 1);
        assert!(hits[0].has_bismillah);
        assert_eq!(hits[0].uthmani_script, "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ");
    }

    #[test]
    fn multi_aya_match_spans_two_ayat() {
        let hits = search(
            "من إفكهم ليقولون ولد الله وإنهم لكاذبون",
            &Aya::new(37, 151).expect("37:151"),
            &tashkeel_opts(6),
        )
        .expect("search must succeed");

        assert_eq!(hits.len(), 1);
        assert_eq!(
            (
                hits[0].start_aya.expect("hit has a start").get().sura_idx,
                hits[0].start_aya.expect("hit has a start").get().aya_idx
            ),
            (37, 151)
        );
        assert_eq!(hits[0].num_ayat, 2);
        assert_eq!(
            hits[0].uthmani_script,
            "مِّنْ إِفْكِهِمْ لَيَقُولُونَ وَلَدَ ٱللَّهُ وَإِنَّهُمْ لَكَـٰذِبُونَ"
        );
    }

    #[test]
    fn match_cutting_a_merged_word_is_an_error() {
        // Verified against Python: it raises `PartOfUthmaniWord` here too.
        let result = search(
            "قَالَ يَا",
            &Aya::new(20, 94).expect("20:94"),
            &tashkeel_opts(4),
        );
        assert!(result.is_err());
    }

    #[test]
    fn window_too_small_yields_no_hits() {
        let hits = search("نستعين", &Aya::new(1, 1).expect("1:1"), &tashkeel_opts(2))
            .expect("search must succeed");
        assert!(hits.is_empty());
    }
}
