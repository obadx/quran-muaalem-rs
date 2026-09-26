//! C2a: Quran navigation (`Aya`) and shared format types.
//!
//! Ports blocks A (containers) and B (navigation) of `utils.py` from
//! `quran-transcript`. Suras and ayat use 1-based numbers (Quranic standard);
//! word indices are 0-based with exclusive ends (Python-style). All text
//! handling is character-based; this crate never byte-indexes Arabic text.

use std::{collections::HashMap, error::Error, fmt};

use crate::{alphabet, quran_data::quran_text};

/// Number of suras in the Holy Quran.
pub const NUM_SURAS: usize = 114;

/// Errors for Quran navigation and normalisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AyaError {
    /// Sura number outside `1..=114`.
    InvalidSura {
        /// The rejected sura number.
        got: usize,
    },
    /// Aya number outside `1..=sura_len`.
    InvalidAya {
        /// The containing sura number.
        sura: usize,
        /// The rejected aya number.
        got: usize,
        /// Number of ayat in the sura.
        max: usize,
    },
    /// `ignore_taa_marboota` and `normalize_taat` set at the same time.
    ConflictingTaaOptions,
    /// `WordSpan` cuts inside a merged Uthmani word.
    PartOfUthmaniWord {
        /// The containing sura number.
        sura: usize,
        /// The containing aya number.
        aya: usize,
        /// Requested start word index.
        start: usize,
        /// Requested end word index (exclusive).
        end: usize,
    },
    /// `WordSpan` reaches past the end of the encoded words.
    ///
    /// Python raises `KeyError` here; a typed error is more robust.
    SpanOutOfRange {
        /// Requested start word index.
        start: usize,
        /// Requested end word index, if bounded.
        end: Option<usize>,
        /// Number of encoded Imlaey words.
        len: usize,
    },
}

impl fmt::Display for AyaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSura { got } => write!(f, "wrong sura index {got} (want 1..=114)"),
            Self::InvalidAya { sura, got, max } => write!(
                f,
                "aya index out of range (sura_index={sura} aya_index={got}) and length of sura={max}"
            ),
            Self::ConflictingTaaOptions => write!(
                f,
                "you can not `ignore_taa_marboota` and `normalize_taat` at the same time"
            ),
            Self::PartOfUthmaniWord {
                sura,
                aya,
                start,
                end,
            } => write!(
                f,
                "the Imlaey word is part of an Uthmani word, sura: `{sura}`, aya: `{aya}`, Imlaey word span: ({start}, {end})"
            ),
            Self::SpanOutOfRange { start, end, len } => write!(
                f,
                "word span ({start}, {end:?}) past the end of {len} encoded words"
            ),
        }
    }
}

impl Error for AyaError {}

/// One word-level Uthmani-to-Imlaey spelling pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasmEntry {
    /// The Uthmani word (or words).
    pub uthmani: String,
    /// The Imlaey word (or words).
    pub imlaey: String,
}

/// Exceptional word mappings split per word, mirroring
/// [`AyaFormat::get_formatted_rasm_map`] in Python.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasmFormat {
    /// Uthmani words per map entry.
    pub uthmani: Vec<Vec<String>>,
    /// Imlaey words per map entry.
    pub imlaey: Vec<Vec<String>>,
}

/// A word span with an exclusive end; `end` of `None` means to the last word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordSpan {
    /// Start word index (inclusive).
    pub start: usize,
    /// End word index (exclusive), or the end of the aya.
    pub end: Option<usize>,
}

/// A bidirectional word index between the two scripts (exclusive boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuranWordIndex {
    /// Word index in Imlaey script.
    pub imlaey: usize,
    /// Word index in Uthmani script.
    pub uthmani: usize,
}

/// One aya in both scripts with its metadata, mirroring `AyaFormat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AyaFormat {
    /// Sura number, 1-based.
    pub sura_idx: usize,
    /// Aya number, 1-based.
    pub aya_idx: usize,
    /// Name of the sura.
    pub sura_name: String,
    /// Number of ayat in the sura.
    pub num_ayat_in_sura: usize,
    /// The aya in Uthmani script.
    pub uthmani: String,
    /// Uthmani words of the aya.
    pub uthmani_words: Vec<String>,
    /// The aya in Imlaey script.
    pub imlaey: String,
    /// Imlaey words of the aya.
    pub imlaey_words: Vec<String>,
    /// Istiaatha in Uthmani script.
    pub istiaatha_uthmani: String,
    /// Istiaatha in Imlaey script.
    pub istiaatha_imlaey: String,
    /// Word-level script map; `None` here because the map asset is not
    /// embedded (see C1). The step-4 alignment engine reconstructs the
    /// mapping from the word lists instead.
    pub rasm_map: Option<Vec<RasmEntry>>,
    /// Bismillah in Uthmani script (`None` unless first aya of a sura
    /// that has one).
    pub bismillah_uthmani: Option<String>,
    /// Bismillah in Imlaey script (`None` unless first aya of a sura
    /// that has one).
    pub bismillah_imlaey: Option<String>,
    /// Word-level Bismillah map; `None` (map asset not embedded).
    pub bismillah_map: Option<Vec<RasmEntry>>,
}

/// Output of the Imlaey-to-Uthmani encoder (step 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingOutput {
    /// Map from Imlaey word indices to Uthmani word indices.
    pub imlaey2uthmani: HashMap<usize, usize>,
    /// Uthmani words (with optional istiaatha/bismillah/sadaka affixes).
    pub uthmani_words: Vec<String>,
    /// Imlaey words (with optional istiaatha/bismillah/sadaka affixes).
    pub imlaey_words: Vec<String>,
    /// `(start, end)` word span of the core aya content.
    pub aya_imlaey_span_words: (usize, usize),
    /// Word span of the istiaatha prefix, if present.
    pub istiaatha_imlaey_span_words: Option<(usize, usize)>,
    /// Word span of the bismillah prefix, if present.
    pub bismillah_imlaey_span_words: Option<(usize, usize)>,
    /// Word span of the sadaka suffix, if present.
    pub sadaka_imlaey_span_words: Option<(usize, usize)>,
}

/// Output of converting one Imlaey span to Uthmani (step 4).
// The four flags mirror Python `Imlaey2uthmaniOutput` one-to-one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Imlaey2uthmaniOutput {
    /// The input Imlaey text segment.
    pub imlaey: String,
    /// The converted Uthmani text.
    pub uthmani: String,
    /// Start of the Quranic content, if any.
    pub quran_start: Option<QuranWordIndex>,
    /// End of the Quranic content, if any.
    pub quran_end: Option<QuranWordIndex>,
    /// Whether the segment contains istiaatha.
    pub has_istiaatha: bool,
    /// Whether the segment contains bismillah.
    pub has_bismillah: bool,
    /// Whether the segment contains sadaka.
    pub has_sadaka: bool,
    /// Whether the segment contains core Quranic text.
    pub has_quran: bool,
}

/// A text segment in both scripts with dual indexing (step 5).
///
/// Note: Python names the Imlaey field `imalaey` (typo); it is spelled
/// correctly here.
// The four flags mirror Python `SegmentScripts` one-to-one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct SegmentScripts {
    /// Full Imlaey script text.
    pub imlaey: String,
    /// Full Uthmani script text.
    pub uthmani: String,
    /// Whether the segment contains istiaatha.
    pub has_istiaatha: bool,
    /// Whether the segment contains bismillah.
    pub has_bismillah: bool,
    /// Whether the segment contains sadaka.
    pub has_sadaka: bool,
    /// Whether the segment contains core Quranic text.
    pub has_quran: bool,
    /// `(sura, aya, word)` start position, if Quranic content exists.
    pub start_span: Option<(usize, usize, QuranWordIndex)>,
    /// `(sura, aya, word)` end position, if Quranic content exists.
    pub end_span: Option<(usize, usize, QuranWordIndex)>,
}

/// Optional affixes for the Imlaey-to-Uthmani encoder.
///
/// Mirrors the `include_istiaatha` / `include_bismillah` / `include_sadaka`
/// flags threaded through `utils.py`. A group struct keeps call sites
/// readable (one flag struct instead of three bare `bool`s).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EncodeFlags {
    /// Prepend istiaatha (only honoured on the first aya of a sura).
    pub istiaatha: bool,
    /// Prepend Bismillah (only honoured where the data has one).
    pub bismillah: bool,
    /// Append sadaka (only honoured on the last aya of a sura).
    pub sadaka: bool,
}

/// A cursor into the Quran, mirroring Python `Aya`.
///
/// Stores 1-based sura/aya numbers. Navigation (`step`, `ayat_after`) wraps
/// around the end of the Quran, exactly like the Python version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// The position of an `Aya` is a sura number plus an aya number; renaming
// either field would only obscure the port.
#[allow(clippy::struct_field_names)]
pub struct Aya {
    sura_no: usize,
    aya_no: usize,
    start_imlaey_word_idx: i32,
}

fn sura_len(sura: usize) -> usize {
    quran_text().quran.surahs[sura - 1].ayahs.len()
}

fn check_indices(sura: usize, aya: usize) -> Result<(), AyaError> {
    if !(1..=NUM_SURAS).contains(&sura) {
        return Err(AyaError::InvalidSura { got: sura });
    }
    let max = sura_len(sura);
    if !(1..=max).contains(&aya) {
        return Err(AyaError::InvalidAya {
            sura,
            got: aya,
            max,
        });
    }
    Ok(())
}

impl Aya {
    /// Creates a cursor at `(sura, aya)`, both 1-based.
    ///
    /// # Errors
    ///
    /// Returns [`AyaError::InvalidSura`] or [`AyaError::InvalidAya`] for
    /// out-of-range indices.
    #[must_use = "the cursor is not registered anywhere"]
    pub fn new(sura: usize, aya: usize) -> Result<Self, AyaError> {
        check_indices(sura, aya)?;
        Ok(Self {
            sura_no: sura,
            aya_no: aya,
            start_imlaey_word_idx: 0,
        })
    }

    /// Sura number, 1-based.
    #[must_use]
    pub fn sura_idx(&self) -> usize {
        self.sura_no
    }

    /// Aya number, 1-based.
    #[must_use]
    pub fn aya_idx(&self) -> usize {
        self.aya_no
    }

    /// Saved Imlaey word offset used by word-based stepping (step 4).
    #[must_use]
    pub fn start_imlaey_word_idx(&self) -> i32 {
        self.start_imlaey_word_idx
    }

    /// Sets the saved Imlaey word offset, mirroring Python `set`.
    pub fn set_start_imlaey_word_idx(&mut self, idx: i32) {
        self.start_imlaey_word_idx = idx;
    }

    /// Moves this cursor to `(sura, aya)`, both 1-based.
    ///
    /// # Errors
    ///
    /// Returns [`AyaError::InvalidSura`] or [`AyaError::InvalidAya`] for
    /// out-of-range indices; the cursor is left unchanged on error.
    pub fn set(&mut self, sura: usize, aya: usize) -> Result<(), AyaError> {
        check_indices(sura, aya)?;
        self.sura_no = sura;
        self.aya_no = aya;
        Ok(())
    }

    /// Returns a new cursor at `(sura, aya)`, both 1-based.
    ///
    /// # Errors
    ///
    /// Returns [`AyaError::InvalidSura`] or [`AyaError::InvalidAya`] for
    /// out-of-range indices.
    #[must_use = "the new cursor is not registered anywhere"]
    pub fn set_new(&self, sura: usize, aya: usize) -> Result<Self, AyaError> {
        let mut new = *self;
        new.set(sura, aya)?;
        new.start_imlaey_word_idx = 0;
        Ok(new)
    }

    /// Returns the aya this cursor points at.
    #[must_use]
    pub fn get(&self) -> AyaFormat {
        let document = quran_text();
        let sura = &document.quran.surahs[self.sura_no - 1];
        let aya = &sura.ayahs[self.aya_no - 1];
        let alphabet = alphabet::quran_alphabet();
        AyaFormat {
            sura_idx: self.sura_no,
            aya_idx: self.aya_no,
            sura_name: sura.name.clone(),
            num_ayat_in_sura: sura.ayahs.len(),
            uthmani: aya.uthmani.clone(),
            uthmani_words: aya.uthmani.split(' ').map(str::to_owned).collect(),
            imlaey: aya.imlaey.clone(),
            imlaey_words: aya.imlaey.split(' ').map(str::to_owned).collect(),
            istiaatha_uthmani: alphabet.istiaatha.uthmani.clone(),
            istiaatha_imlaey: alphabet.istiaatha.imlaey.clone(),
            rasm_map: None,
            bismillah_uthmani: aya.bismillah_uthmani.clone(),
            bismillah_imlaey: aya.bismillah_imlaey.clone(),
            bismillah_map: None,
        }
    }

    /// Whether this is the last aya of its sura.
    #[must_use]
    pub fn is_last(&self) -> bool {
        self.aya_no == self.get().num_ayat_in_sura
    }

    /// Returns the cursor `steps` ayat after (or before, if negative).
    ///
    /// Wraps around the ends of the Quran, exactly like Python `step`.
    #[must_use]
    pub fn step(&self, steps: i32) -> Self {
        let mut sura_no = self.sura_no;
        let mut aya_no = self.aya_no;
        if steps >= 0 {
            for _ in 0..steps {
                aya_no += 1;
                if aya_no > sura_len(sura_no) {
                    aya_no = 1;
                    sura_no = sura_no % NUM_SURAS + 1;
                }
            }
        } else {
            for _ in steps..0 {
                if aya_no > 1 {
                    aya_no -= 1;
                } else {
                    sura_no = (sura_no + NUM_SURAS - 2) % NUM_SURAS + 1;
                    aya_no = sura_len(sura_no);
                }
            }
        }
        Self {
            sura_no,
            aya_no,
            start_imlaey_word_idx: 0,
        }
    }

    /// Iterates over ayat starting at this cursor.
    ///
    /// With `Some(n)` yields `n` ayat with wraparound (mirroring Python
    /// `get_ayat_after(num_ayat=n)`); with `None` yields up to and including
    /// the last aya of the Quran without wrapping.
    pub fn ayat_after(&self, num_ayat: Option<usize>) -> impl Iterator<Item = Self> {
        let mut current = Some(*self);
        let mut remaining = num_ayat;
        std::iter::from_fn(move || {
            if remaining == Some(0) {
                return None;
            }
            let out = current.take()?;
            if let Some(left) = remaining.as_mut() {
                // Bounded mode wraps around the whole Quran like Python.
                *left -= 1;
                current = Some(out.step(1));
            } else if out.sura_no == NUM_SURAS && out.aya_no == sura_len(out.sura_no) {
                // Unbounded mode stops at the last aya without wrapping.
            } else {
                current = Some(out.step(1));
            }
            Some(out)
        })
    }
}

/// Saturating `usize` → `i32` for word counts (always tiny in practice).
pub(crate) fn i32_len(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Whether two optional exclusive `(start, end)` spans overlap.
fn has_intersection(x: Option<(usize, usize)>, y: Option<(usize, usize)>) -> bool {
    match (x, y) {
        (Some((x0, x1)), Some((y0, y1))) => x0.max(y0) < x1.min(y1),
        _ => false,
    }
}

impl Aya {
    /// Builds the parallel Imlaey/Uthmani word lists and their index map.
    ///
    /// Handles the length-mismatch cases from the alphabet data: unique-rasm
    /// pairs (`يَبْنَؤُمَّ` ↔ `يَا ابْنَ أُمَّ`) checked first, then
    /// `imlaey_starts` (`يَا`, `هَا`, `وَيَا` map two Imlaey words to one
    /// Uthmani word). Requested affixes at invalid positions are skipped
    /// (Python emits a warning instead).
    fn encode_imlaey_to_uthmani(&self, flags: EncodeFlags) -> EncodingOutput {
        let alphabet = alphabet::quran_alphabet();
        let view = self.get();
        let mut uthmani_words: Vec<String> = Vec::new();
        let mut imlaey_words: Vec<String> = Vec::new();
        let mut istiaatha_span: Option<(usize, usize)> = None;
        let mut bismillah_span: Option<(usize, usize)> = None;
        let mut sadaka_span: Option<(usize, usize)> = None;

        if flags.istiaatha && self.aya_no == 1 {
            let start = imlaey_words.len();
            uthmani_words.extend(alphabet.istiaatha.uthmani.split(' ').map(str::to_owned));
            imlaey_words.extend(alphabet.istiaatha.imlaey.split(' ').map(str::to_owned));
            istiaatha_span = Some((start, imlaey_words.len()));
        }
        if flags.bismillah
            && let (Some(uth), Some(iml)) = (
                view.bismillah_uthmani.as_deref(),
                view.bismillah_imlaey.as_deref(),
            )
        {
            let start = imlaey_words.len();
            uthmani_words.extend(uth.split(' ').map(str::to_owned));
            imlaey_words.extend(iml.split(' ').map(str::to_owned));
            bismillah_span = Some((start, imlaey_words.len()));
        }
        let aya_start = imlaey_words.len();
        uthmani_words.extend(view.uthmani_words.iter().cloned());
        imlaey_words.extend(view.imlaey_words.iter().cloned());
        let aya_span = (aya_start, imlaey_words.len());

        if flags.sadaka && self.aya_no == view.num_ayat_in_sura {
            let start = imlaey_words.len();
            uthmani_words.extend(alphabet.sadaka.uthmani.split(' ').map(str::to_owned));
            imlaey_words.extend(alphabet.sadaka.imlaey.split(' ').map(str::to_owned));
            sadaka_span = Some((start, imlaey_words.len()));
        }

        let imlaey2uthmani = if uthmani_words.len() == imlaey_words.len() {
            (0..uthmani_words.len()).map(|idx| (idx, idx)).collect()
        } else {
            let mut map = HashMap::with_capacity(imlaey_words.len());
            let starts = &alphabet.unique_rasm_map.imlaey_starts;
            let mut iml_idx = 0;
            for (uth_idx, _) in uthmani_words.iter().enumerate() {
                if let Some(span) = Self::unique_rasm_span(iml_idx, &imlaey_words) {
                    for key in iml_idx..iml_idx + span {
                        map.insert(key, uth_idx);
                    }
                    iml_idx += span;
                } else if starts.contains(&imlaey_words[iml_idx]) {
                    map.insert(iml_idx, uth_idx);
                    map.insert(iml_idx + 1, uth_idx);
                    iml_idx += 2;
                } else {
                    map.insert(iml_idx, uth_idx);
                    iml_idx += 1;
                }
            }
            debug_assert_eq!(iml_idx, imlaey_words.len());
            map
        };
        debug_assert_eq!(imlaey2uthmani.len(), imlaey_words.len());

        EncodingOutput {
            imlaey2uthmani,
            uthmani_words,
            imlaey_words,
            aya_imlaey_span_words: aya_span,
            istiaatha_imlaey_span_words: istiaatha_span,
            bismillah_imlaey_span_words: bismillah_span,
            sadaka_imlaey_span_words: sadaka_span,
        }
    }

    /// Length of the `unique_rasm_map` Imlaey phrase starting at `idx`, if any.
    fn unique_rasm_span(idx: usize, words: &[String]) -> Option<usize> {
        for pair in &alphabet::quran_alphabet().unique_rasm_map.rasm_map {
            let span = pair.imlaey.split(' ').count();
            let end = idx.checked_add(span)?;
            if words
                .get(idx..end)
                .is_some_and(|window| window.join(" ") == pair.imlaey)
            {
                return Some(span);
            }
        }
        None
    }

    /// Converts one Imlaey word span to Uthmani text.
    ///
    /// Rejects spans cutting inside a merged Uthmani word. The trailing
    /// space in outputs like `"قَالَ يَبْنَؤُمَّ "` is faithful Python
    /// behaviour (a space follows every emitted word except at `end - 1`).
    /// Empty or inverted spans yield an empty string.
    fn decode_uthmani(
        &self,
        span: &WordSpan,
        map: &HashMap<usize, usize>,
        uthmani_words: &[String],
    ) -> Result<String, AyaError> {
        let len = map.len();
        let end = span.end.unwrap_or(len);
        if span.start > len || end > len || span.start > end {
            return Err(AyaError::SpanOutOfRange {
                start: span.start,
                end: span.end,
                len,
            });
        }
        if span.start == end {
            return Ok(String::new());
        }
        if map.contains_key(&end) && map[&(end - 1)] == map[&end] {
            return Err(AyaError::PartOfUthmaniWord {
                sura: self.sura_no,
                aya: self.aya_no,
                start: span.start,
                end,
            });
        }
        if span.start > 0 && map[&span.start] == map[&(span.start - 1)] {
            return Err(AyaError::PartOfUthmaniWord {
                sura: self.sura_no,
                aya: self.aya_no,
                start: span.start,
                end,
            });
        }

        let mut out = String::new();
        let mut prev: Option<usize> = None;
        for idx in span.start..end {
            let uth_idx = map[&idx];
            if prev != Some(uth_idx) {
                out.push_str(&uthmani_words[uth_idx]);
                if idx != end - 1 {
                    out.push(' ');
                }
            }
            prev = Some(uth_idx);
        }
        Ok(out)
    }

    /// Returns the Uthmani script of an Imlaey word span.
    ///
    /// # Errors
    ///
    /// Returns [`AyaError::PartOfUthmaniWord`] when the span cuts inside a
    /// merged Uthmani word, or [`AyaError::SpanOutOfRange`] past the end.
    pub fn imlaey_to_uthmani(
        &self,
        span: WordSpan,
        flags: EncodeFlags,
    ) -> Result<String, AyaError> {
        let encoding = self.encode_imlaey_to_uthmani(flags);
        self.decode_uthmani(&span, &encoding.imlaey2uthmani, &encoding.uthmani_words)
    }

    /// Converts a span and reports which parts are Quranic content.
    ///
    /// Mirrors `imlaey_to_uthmani(..., return_checks=True)`.
    ///
    /// # Errors
    ///
    /// Same as [`Aya::imlaey_to_uthmani`].
    pub fn imlaey_to_uthmani_checked(
        &self,
        span: WordSpan,
        flags: EncodeFlags,
    ) -> Result<Imlaey2uthmaniOutput, AyaError> {
        let encoding = self.encode_imlaey_to_uthmani(flags);
        let uthmani =
            self.decode_uthmani(&span, &encoding.imlaey2uthmani, &encoding.uthmani_words)?;
        let end = span.end.unwrap_or(encoding.imlaey_words.len());
        let input = (span.start, end);
        let has_quran = has_intersection(Some(input), Some(encoding.aya_imlaey_span_words));
        let (quran_start, quran_end) = if has_quran {
            let (span_start, span_end) = encoding.aya_imlaey_span_words;
            let rel_start = span.start.saturating_sub(span_start);
            let rel_end = end.saturating_sub(span_start).min(span_end - span_start);
            // NOTE: the relative indices below index the global map verbatim
            // from Python; with affixes plus an offset start this reports the
            // affix position (upstream quirk, kept for equivalence).
            Some((
                QuranWordIndex {
                    imlaey: rel_start,
                    uthmani: encoding.imlaey2uthmani[&rel_start],
                },
                QuranWordIndex {
                    imlaey: rel_end,
                    uthmani: encoding.imlaey2uthmani[&(rel_end - 1)] + 1,
                },
            ))
        } else {
            None
        }
        .unzip();

        Ok(Imlaey2uthmaniOutput {
            imlaey: encoding.imlaey_words[span.start..end].join(" "),
            uthmani,
            quran_start,
            quran_end,
            has_quran,
            has_istiaatha: has_intersection(Some(input), encoding.istiaatha_imlaey_span_words),
            has_bismillah: has_intersection(Some(input), encoding.bismillah_imlaey_span_words),
            has_sadaka: has_intersection(Some(input), encoding.sadaka_imlaey_span_words),
        })
    }

    /// Fetches `window` Imlaey words from `start` (may be negative) in both
    /// scripts, crossing aya boundaries with wraparound.
    ///
    /// # Errors
    ///
    /// Propagates [`AyaError`] when an internal span cuts a merged word
    /// (mirrors Python raising `PartOfUthmaniWord` out of this method).
    pub fn get_by_imlaey_words(
        &self,
        start: i32,
        window: i32,
        flags: EncodeFlags,
    ) -> Result<SegmentScripts, AyaError> {
        let mut start_aya = *self;
        let mut start = start + self.start_imlaey_word_idx;
        if start < 0 {
            let mut pos = start;
            while pos < 0 {
                start_aya = start_aya.step(-1);
                pos += i32_len(start_aya.encode_imlaey_to_uthmani(flags).imlaey_words.len());
            }
            start = pos;
        }
        loop {
            let len = i32_len(start_aya.encode_imlaey_to_uthmani(flags).imlaey_words.len());
            if start < len {
                break;
            }
            start -= len;
            start_aya = start_aya.step(1);
        }

        let mut imlaey_str = String::new();
        let mut uthmani_str = String::new();
        let mut has_quran = false;
        let mut has_istiaatha = false;
        let mut has_bismillah = false;
        let mut has_sadaka = false;
        let mut first_time = true;
        let mut quran_word_end: Option<QuranWordIndex> = None;
        let start_sura = start_aya.sura_idx();
        let start_aya_idx = start_aya.aya_idx();
        let mut start_span: Option<(usize, usize, QuranWordIndex)> = None;
        let mut loop_aya = start_aya;
        let mut remaining = window;
        let mut end_sura = start_sura;
        let mut end_aya = start_aya_idx;
        while remaining > 0 {
            if !imlaey_str.is_empty() {
                imlaey_str.push(' ');
            }
            if !uthmani_str.is_empty() {
                uthmani_str.push(' ');
            }
            let encoded_len = i32_len(loop_aya.encode_imlaey_to_uthmani(flags).imlaey_words.len());
            let end = (start + remaining).min(encoded_len);
            let span = WordSpan {
                start: usize::try_from(start).unwrap_or(0),
                end: Some(usize::try_from(end).unwrap_or(0)),
            };
            let checked = loop_aya.imlaey_to_uthmani_checked(span, flags)?;
            if first_time && checked.has_quran {
                first_time = false;
                if let Some(quran_start) = checked.quran_start {
                    start_span = Some((start_sura, start_aya_idx, quran_start));
                }
            }
            imlaey_str.push_str(&checked.imlaey);
            uthmani_str.push_str(&checked.uthmani);
            has_quran = has_quran || checked.has_quran;
            has_istiaatha = has_istiaatha || checked.has_istiaatha;
            has_bismillah = has_bismillah || checked.has_bismillah;
            has_sadaka = has_sadaka || checked.has_sadaka;
            if checked.has_quran {
                quran_word_end = checked.quran_end;
                end_sura = loop_aya.sura_idx();
                end_aya = loop_aya.aya_idx();
            }
            debug_assert!(end > start);
            remaining -= end - start;
            loop_aya = loop_aya.step(1);
            start = 0;
        }
        let end_span = match (has_quran, quran_word_end) {
            (true, Some(word_end)) => Some((end_sura, end_aya, word_end)),
            _ => None,
        };

        Ok(SegmentScripts {
            imlaey: imlaey_str,
            uthmani: uthmani_str,
            has_istiaatha,
            has_bismillah,
            has_sadaka,
            has_quran,
            start_span,
            end_span,
        })
    }

    /// Moves to a new cursor by Imlaey word offset.
    ///
    /// `start + window` words past the saved word offset (both may shift
    /// across ayat with wraparound); the landing word offset is stored on
    /// the returned cursor.
    #[must_use]
    pub fn step_by_imlaey_words(&self, start: i32, window: i32, flags: EncodeFlags) -> Self {
        let mut pos = self.start_imlaey_word_idx + start + window;
        let mut loop_aya = *self;
        if pos < 0 {
            while pos < 0 {
                loop_aya = loop_aya.step(-1);
                pos += i32_len(loop_aya.encode_imlaey_to_uthmani(flags).imlaey_words.len());
            }
        } else {
            loop {
                let len = i32_len(loop_aya.encode_imlaey_to_uthmani(flags).imlaey_words.len());
                if pos < len {
                    break;
                }
                pos -= len;
                loop_aya = loop_aya.step(1);
            }
        }
        let mut out = loop_aya;
        out.start_imlaey_word_idx = pos;
        out
    }
}

impl fmt::Display for Aya {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Aya(sura_idx={}, aya_idx={})", self.sura_no, self.aya_no)
    }
}

#[cfg(test)]
// Expected strings are byte-exact Quranic corpus text, which is not
// NFC-normalised (combining-mark order is significant here).
#[allow(clippy::unicode_not_nfc)]
mod tests {
    use super::{Aya, AyaError};

    #[test]
    fn get_returns_the_expected_fatiha_view() {
        let aya = Aya::new(1, 1).expect("1:1 must exist");
        let view = aya.get();

        assert_eq!(view.sura_idx, 1);
        assert_eq!(view.aya_idx, 1);
        assert_eq!(view.sura_name, "الفاتحة");
        assert_eq!(view.num_ayat_in_sura, 7);
        assert!(!view.uthmani.is_empty());
        assert!(!view.imlaey.is_empty());
        assert_eq!(view.uthmani_words.join(" "), view.uthmani);
        assert_eq!(view.imlaey_words.join(" "), view.imlaey);
        assert!(!view.istiaatha_uthmani.is_empty());
        assert!(!view.istiaatha_imlaey.is_empty());
        // Fatiha's Bismillah is an aya, so no separate prefix.
        assert!(view.bismillah_uthmani.is_none());
        assert!(view.bismillah_imlaey.is_none());
        assert!(view.rasm_map.is_none());
    }

    #[test]
    fn bismillah_prefix_only_on_first_ayat() {
        assert!(
            Aya::new(2, 1)
                .expect("2:1")
                .get()
                .bismillah_uthmani
                .is_some()
        );
        assert!(
            Aya::new(2, 1)
                .expect("2:1")
                .get()
                .bismillah_imlaey
                .is_some()
        );
        assert!(
            Aya::new(2, 2)
                .expect("2:2")
                .get()
                .bismillah_uthmani
                .is_none()
        );
        // Tawbah has no Bismillah at all.
        assert!(
            Aya::new(9, 1)
                .expect("9:1")
                .get()
                .bismillah_uthmani
                .is_none()
        );
    }

    #[test]
    fn constructor_rejects_out_of_range_indices() {
        assert_eq!(Aya::new(0, 1), Err(AyaError::InvalidSura { got: 0 }));
        assert_eq!(Aya::new(115, 1), Err(AyaError::InvalidSura { got: 115 }));
        assert_eq!(
            Aya::new(1, 8),
            Err(AyaError::InvalidAya {
                sura: 1,
                got: 8,
                max: 7
            })
        );
        assert_eq!(
            Aya::new(114, 7),
            Err(AyaError::InvalidAya {
                sura: 114,
                got: 7,
                max: 6
            })
        );
    }

    #[test]
    fn step_moves_across_ayat_suras_and_wraps() {
        let first = Aya::new(1, 1).expect("1:1");
        assert_eq!(first.step(0), first);
        assert_eq!(first.step(1), Aya::new(1, 2).expect("1:2"));
        assert_eq!(
            Aya::new(1, 7).expect("1:7").step(1),
            Aya::new(2, 1).expect("2:1")
        );
        assert_eq!(
            Aya::new(2, 1).expect("2:1").step(-1),
            Aya::new(1, 7).expect("1:7")
        );
        assert_eq!(
            Aya::new(114, 6).expect("114:6").step(1),
            Aya::new(1, 1).expect("1:1")
        );
        assert_eq!(
            Aya::new(1, 1).expect("1:1").step(-1),
            Aya::new(114, 6).expect("114:6")
        );
        // A full Quran cycle is 6236 ayat.
        assert_eq!(first.step(6236), first);
        assert_eq!(first.step(-6236), first);
    }

    #[test]
    fn set_and_set_new_update_positions() {
        let mut aya = Aya::new(1, 1).expect("1:1");
        aya.set(114, 2).expect("114:2 must exist");
        assert_eq!((aya.sura_idx(), aya.aya_idx()), (114, 2));
        assert!(aya.set(114, 7).is_err());
        // Failed `set` leaves the cursor unchanged.
        assert_eq!((aya.sura_idx(), aya.aya_idx()), (114, 2));

        let branched = aya.set_new(4, 4).expect("4:4 must exist");
        assert_eq!((branched.sura_idx(), branched.aya_idx()), (4, 4));
        assert_eq!((aya.sura_idx(), aya.aya_idx()), (114, 2));
        assert!(aya.set_new(0, 1).is_err());
    }

    #[test]
    fn is_last_detects_sura_ends() {
        assert!(Aya::new(1, 7).expect("1:7").is_last());
        assert!(!Aya::new(1, 1).expect("1:1").is_last());
        assert!(Aya::new(114, 6).expect("114:6").is_last());
    }

    #[test]
    fn ayat_after_yields_bounded_windows_with_wraparound() {
        let start = Aya::new(114, 5).expect("114:5");
        let collected: Vec<Aya> = start.ayat_after(Some(10)).collect();

        assert_eq!(collected.len(), 10);
        assert_eq!(collected[0], start);
        assert_eq!(collected[1], Aya::new(114, 6).expect("114:6"));
        assert_eq!(collected[2], Aya::new(1, 1).expect("1:1"));
        assert_eq!(
            *collected.last().expect("non-empty"),
            Aya::new(2, 1).expect("2:1")
        );
    }

    #[test]
    fn ayat_after_without_bound_runs_to_the_end_of_the_quran() {
        let from_start: Vec<Aya> = Aya::new(1, 1).expect("1:1").ayat_after(None).collect();
        assert_eq!(from_start.len(), 6236);
        assert_eq!(
            *from_start.last().expect("non-empty"),
            Aya::new(114, 6).expect("114:6")
        );

        let from_end: Vec<Aya> = Aya::new(114, 6).expect("114:6").ayat_after(None).collect();
        assert_eq!(from_end.len(), 1);
    }

    // ------------------------------------------------------------------
    // Step 4: alignment engine. Vectors transcribed from `aya_pytest.py`
    // (`test_imlaey_to_uthmani`, exception/caching tests,
    // `test_get_by_imlaey_words`, `test_step_by_imlaey_words`).
    // ------------------------------------------------------------------

    use super::{EncodeFlags, QuranWordIndex, SegmentScripts, WordSpan};

    /// One row of the `test_imlaey_to_uthmani` vectors: sura, aya, span
    /// start/end, affix flags, expected Uthmani text.
    type ConversionCase = (
        usize,
        usize,
        usize,
        Option<usize>,
        bool,
        bool,
        bool,
        &'static str,
    );
    /// One row of the `test_get_by_imlaey_words` vectors.
    type SegmentCase = (usize, usize, i32, i32, bool, bool, bool, SegmentScripts);

    fn flags(istiaatha: bool, bismillah: bool, sadaka: bool) -> EncodeFlags {
        EncodeFlags {
            istiaatha,
            bismillah,
            sadaka,
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn imlaey_to_uthmani_matches_the_python_vectors() {
        let cases: [ConversionCase; 17] = [
            (
                1,
                1,
                0,
                None,
                false,
                false,
                false,
                "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
            ),
            (
                1,
                1,
                0,
                None,
                true,
                false,
                false,
                "أَعُوذُ بِٱللَّهِ مِنَ ٱلشَّيْطَانِ ٱلرَّجِيمِ بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
            ),
            (
                1,
                7,
                0,
                None,
                false,
                false,
                true,
                "صِرَٰطَ ٱلَّذِينَ أَنْعَمْتَ عَلَيْهِمْ غَيْرِ ٱلْمَغْضُوبِ عَلَيْهِمْ وَلَا ٱلضَّآلِّينَ صَدَقَ ٱللَّهُ ٱلْعَظِيمُ",
            ),
            (
                1,
                7,
                3,
                Some(8),
                false,
                false,
                true,
                "عَلَيْهِمْ غَيْرِ ٱلْمَغْضُوبِ عَلَيْهِمْ وَلَا",
            ),
            (1, 7, 8, Some(10), false, false, true, "ٱلضَّآلِّينَ صَدَقَ"),
            (
                2,
                21,
                0,
                None,
                false,
                false,
                false,
                "يَـٰٓأَيُّهَا ٱلنَّاسُ ٱعْبُدُوا۟ رَبَّكُمُ ٱلَّذِى خَلَقَكُمْ وَٱلَّذِينَ مِن قَبْلِكُمْ لَعَلَّكُمْ تَتَّقُونَ",
            ),
            // Affix flags on a non-first aya are ignored, like Python.
            (
                2,
                21,
                0,
                None,
                true,
                true,
                false,
                "يَـٰٓأَيُّهَا ٱلنَّاسُ ٱعْبُدُوا۟ رَبَّكُمُ ٱلَّذِى خَلَقَكُمْ وَٱلَّذِينَ مِن قَبْلِكُمْ لَعَلَّكُمْ تَتَّقُونَ",
            ),
            (
                20,
                94,
                0,
                None,
                false,
                false,
                false,
                "قَالَ يَبْنَؤُمَّ لَا تَأْخُذْ بِلِحْيَتِى وَلَا بِرَأْسِىٓ إِنِّى خَشِيتُ أَن تَقُولَ فَرَّقْتَ بَيْنَ بَنِىٓ إِسْرَٰٓءِيلَ وَلَمْ تَرْقُبْ قَوْلِى",
            ),
            (20, 94, 0, Some(5), false, false, false, "قَالَ يَبْنَؤُمَّ لَا"),
            (
                22,
                56,
                0,
                None,
                false,
                false,
                false,
                "ٱلْمُلْكُ يَوْمَئِذٍۢ لِّلَّهِ يَحْكُمُ بَيْنَهُمْ فَٱلَّذِينَ ءَامَنُوا۟ وَعَمِلُوا۟ ٱلصَّـٰلِحَـٰتِ فِى جَنَّـٰتِ ٱلنَّعِيمِ",
            ),
            (
                22,
                56,
                3,
                Some(7),
                false,
                false,
                false,
                "يَحْكُمُ بَيْنَهُمْ فَٱلَّذِينَ ءَامَنُوا۟",
            ),
            (
                2,
                31,
                0,
                None,
                false,
                false,
                false,
                "وَعَلَّمَ ءَادَمَ ٱلْأَسْمَآءَ كُلَّهَا ثُمَّ عَرَضَهُمْ عَلَى ٱلْمَلَـٰٓئِكَةِ فَقَالَ أَنۢبِـُٔونِى بِأَسْمَآءِ هَـٰٓؤُلَآءِ إِن كُنتُمْ صَـٰدِقِينَ",
            ),
            (
                2,
                31,
                9,
                None,
                false,
                false,
                false,
                "أَنۢبِـُٔونِى بِأَسْمَآءِ هَـٰٓؤُلَآءِ إِن كُنتُمْ صَـٰدِقِينَ",
            ),
            (
                47,
                38,
                0,
                None,
                false,
                false,
                false,
                "هَـٰٓأَنتُمْ هَـٰٓؤُلَآءِ تُدْعَوْنَ لِتُنفِقُوا۟ فِى سَبِيلِ ٱللَّهِ فَمِنكُم مَّن يَبْخَلُ وَمَن يَبْخَلْ فَإِنَّمَا يَبْخَلُ عَن نَّفْسِهِۦ وَٱللَّهُ ٱلْغَنِىُّ وَأَنتُمُ ٱلْفُقَرَآءُ وَإِن تَتَوَلَّوْا۟ يَسْتَبْدِلْ قَوْمًا غَيْرَكُمْ ثُمَّ لَا يَكُونُوٓا۟ أَمْثَـٰلَكُم",
            ),
            (
                47,
                38,
                0,
                Some(5),
                false,
                false,
                false,
                "هَـٰٓأَنتُمْ هَـٰٓؤُلَآءِ تُدْعَوْنَ لِتُنفِقُوا۟",
            ),
            (
                72,
                16,
                0,
                None,
                false,
                false,
                false,
                "وَأَلَّوِ ٱسْتَقَـٰمُوا۟ عَلَى ٱلطَّرِيقَةِ لَأَسْقَيْنَـٰهُم مَّآءً غَدَقًۭا",
            ),
            (72, 16, 0, Some(4), false, false, false, "وَأَلَّوِ ٱسْتَقَـٰمُوا۟ عَلَى"),
        ];
        for (sura, aya, start, end, istiaatha, bismillah, sadaka, expected) in cases {
            let aya = Aya::new(sura, aya).expect("vector aya must exist");
            let span = WordSpan { start, end };
            assert_eq!(
                aya.imlaey_to_uthmani(span, flags(istiaatha, bismillah, sadaka)),
                Ok(expected.to_owned()),
                "mismatch at {sura}:{aya} span ({start}, {end:?})"
            );
        }
    }

    #[test]
    fn merged_word_spans_are_rejected_on_both_sides() {
        // 68:31 maps Imlaey `يَا وَيْلَنَا` onto Uthmani `يَـٰوَيْلَنَآ`.
        let aya = Aya::new(68, 31).expect("68:31 must exist");
        assert!(matches!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 0,
                    end: Some(2)
                },
                EncodeFlags::default()
            ),
            Err(AyaError::PartOfUthmaniWord { .. })
        ));
        assert!(matches!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 2,
                    end: Some(6)
                },
                EncodeFlags::default()
            ),
            Err(AyaError::PartOfUthmaniWord { .. })
        ));
        // 20:94 maps `يَا ابْنَ أُمَّ` onto `يَبْنَؤُمَّ`.
        let aya = Aya::new(20, 94).expect("20:94 must exist");
        assert!(matches!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 1,
                    end: Some(2)
                },
                EncodeFlags::default()
            ),
            Err(AyaError::PartOfUthmaniWord { .. })
        ));
        assert!(matches!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 2,
                    end: Some(5)
                },
                EncodeFlags::default()
            ),
            Err(AyaError::PartOfUthmaniWord { .. })
        ));
    }

    #[test]
    fn degenerate_spans_are_total() {
        let aya = Aya::new(1, 1).expect("1:1 must exist");
        let none = EncodeFlags::default();
        assert_eq!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 0,
                    end: Some(0)
                },
                none
            ),
            Ok(String::new())
        );
        assert_eq!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 2,
                    end: Some(2)
                },
                none
            ),
            Ok(String::new())
        );
        assert!(matches!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 0,
                    end: Some(99)
                },
                none
            ),
            Err(AyaError::SpanOutOfRange { .. })
        ));
        assert!(matches!(
            aya.imlaey_to_uthmani(
                WordSpan {
                    start: 99,
                    end: None
                },
                none
            ),
            Err(AyaError::SpanOutOfRange { .. })
        ));
    }

    #[test]
    fn encoder_maps_match_the_python_index_tables() {
        // 20:94: `يَا ابْنَ أُمَّ` (1, 2, 3) all point at `يَبْنَؤُمَّ` (1).
        let aya = Aya::new(20, 94).expect("20:94 must exist");
        let encoding = aya.encode_imlaey_to_uthmani(EncodeFlags::default());
        assert_eq!(encoding.uthmani_words.len(), 18);
        assert_eq!(encoding.imlaey_words.len(), 20);
        for (imlaey, uthmani) in [(0, 0), (1, 1), (2, 1), (3, 1), (4, 2), (5, 3)] {
            assert_eq!(encoding.imlaey2uthmani[&imlaey], uthmani);
        }
        assert_eq!(encoding.aya_imlaey_span_words, (0, 20));

        // 47:38: `هَا أَنتُمْ` (0, 1) point at `هَـٰٓأَنتُمْ` (0).
        let aya = Aya::new(47, 38).expect("47:38 must exist");
        let encoding = aya.encode_imlaey_to_uthmani(EncodeFlags::default());
        assert_eq!(encoding.imlaey2uthmani[&0], 0);
        assert_eq!(encoding.imlaey2uthmani[&1], 0);
        assert_eq!(encoding.imlaey2uthmani[&2], 1);

        // 72:16: `وَأَن لَّوِ` (0, 1) point at `وَأَلَّوِ` (0).
        let aya = Aya::new(72, 16).expect("72:16 must exist");
        let encoding = aya.encode_imlaey_to_uthmani(EncodeFlags::default());
        assert_eq!(encoding.imlaey2uthmani[&0], 0);
        assert_eq!(encoding.imlaey2uthmani[&1], 0);
        assert_eq!(encoding.imlaey2uthmani[&2], 1);
    }

    #[test]
    fn affix_spans_track_istiaatha_bismillah_and_sadaka() {
        let none = EncodeFlags::default();

        // Istiaatha is honoured on the first aya only.
        let aya = Aya::new(1, 1).expect("1:1 must exist");
        let with = EncodeFlags {
            istiaatha: true,
            ..none
        };
        let encoding = aya.encode_imlaey_to_uthmani(with);
        assert_eq!(encoding.istiaatha_imlaey_span_words, Some((0, 5)));
        assert_eq!(encoding.aya_imlaey_span_words, (5, 9));
        let aya = Aya::new(1, 2).expect("1:2 must exist");
        assert_eq!(
            aya.encode_imlaey_to_uthmani(with)
                .istiaatha_imlaey_span_words,
            None
        );

        // Bismillah is honoured where the data carries it.
        let with = EncodeFlags {
            bismillah: true,
            ..none
        };
        let aya = Aya::new(2, 1).expect("2:1 must exist");
        let encoding = aya.encode_imlaey_to_uthmani(with);
        assert_eq!(encoding.bismillah_imlaey_span_words, Some((0, 4)));
        assert_eq!(encoding.aya_imlaey_span_words, (4, 5));
        let aya = Aya::new(1, 1).expect("1:1 must exist");
        assert_eq!(
            aya.encode_imlaey_to_uthmani(with)
                .bismillah_imlaey_span_words,
            None
        );
        let aya = Aya::new(9, 1).expect("9:1 must exist");
        assert_eq!(
            aya.encode_imlaey_to_uthmani(with)
                .bismillah_imlaey_span_words,
            None
        );

        // Sadaka is honoured on the last aya only.
        let with = EncodeFlags {
            sadaka: true,
            ..none
        };
        let aya = Aya::new(114, 6).expect("114:6 must exist");
        let encoding = aya.encode_imlaey_to_uthmani(with);
        assert_eq!(encoding.sadaka_imlaey_span_words, Some((3, 6)));
        let aya = Aya::new(114, 5).expect("114:5 must exist");
        assert_eq!(
            aya.encode_imlaey_to_uthmani(with).sadaka_imlaey_span_words,
            None
        );
    }

    #[test]
    fn checked_conversion_reports_quran_and_affix_flags() {
        let aya = Aya::new(20, 94).expect("20:94 must exist");
        let out = aya
            .imlaey_to_uthmani_checked(
                WordSpan {
                    start: 0,
                    end: Some(4),
                },
                EncodeFlags::default(),
            )
            .expect("valid span");
        assert_eq!(out.imlaey, "قَالَ يَا ابْنَ أُمَّ");
        // Faithful trailing space, as in Python.
        assert_eq!(out.uthmani, "قَالَ يَبْنَؤُمَّ ");
        assert_eq!(
            out.quran_start,
            Some(QuranWordIndex {
                imlaey: 0,
                uthmani: 0
            })
        );
        assert_eq!(
            out.quran_end,
            Some(QuranWordIndex {
                imlaey: 4,
                uthmani: 2
            })
        );
        assert!(out.has_quran);
        assert!(!out.has_istiaatha);

        let aya = Aya::new(1, 1).expect("1:1 must exist");
        let out = aya
            .imlaey_to_uthmani_checked(
                WordSpan {
                    start: 0,
                    end: Some(4),
                },
                EncodeFlags {
                    istiaatha: true,
                    ..EncodeFlags::default()
                },
            )
            .expect("valid span");
        assert_eq!(out.quran_start, None);
        assert_eq!(out.quran_end, None);
        assert!(!out.has_quran);
        assert!(out.has_istiaatha);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn get_by_imlaey_words_matches_the_python_vectors() {
        let cases: [SegmentCase; 9] = [
            // Sadaka with no Quran.
            (
                112, 4, 6, 2, false, false, true,
                SegmentScripts {
                    imlaey: "اللَّهُ الْعَظِيمُ".to_owned(),
                    uthmani: "ٱللَّهُ ٱلْعَظِيمُ".to_owned(),
                    has_quran: false, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: true,
                    start_span: None, end_span: None,
                },
            ),
            // Istiaatha with no Quran.
            (
                50, 1, 0, 5, true, true, false,
                SegmentScripts {
                    imlaey: "أَعُوذُ بِاللَّهِ مِنَ الشَّيْطَانِ الرَّجِيمِ".to_owned(),
                    uthmani: "أَعُوذُ بِٱللَّهِ مِنَ ٱلشَّيْطَانِ ٱلرَّجِيمِ".to_owned(),
                    has_quran: false, has_istiaatha: true,
                    has_bismillah: false, has_sadaka: false,
                    start_span: None, end_span: None,
                },
            ),
            // Bismillah with no Quran.
            (
                20, 1, 0, 4, false, true, false,
                SegmentScripts {
                    imlaey: "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ".to_owned(),
                    uthmani: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ".to_owned(),
                    has_quran: false, has_istiaatha: false,
                    has_bismillah: true, has_sadaka: false,
                    start_span: None, end_span: None,
                },
            ),
            // Sadaka after Quran.
            (
                114, 6, 0, 5, false, false, true,
                SegmentScripts {
                    imlaey: "مِنَ الْجِنَّةِ وَالنَّاسِ صَدَقَ اللَّهُ".to_owned(),
                    uthmani: "مِنَ ٱلْجِنَّةِ وَٱلنَّاسِ صَدَقَ ٱللَّهُ".to_owned(),
                    has_quran: true, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: true,
                    start_span: Some((114, 6, QuranWordIndex { imlaey: 0, uthmani: 0 })),
                    end_span: Some((114, 6, QuranWordIndex { imlaey: 3, uthmani: 3 })),
                },
            ),
            // Plain single-aya window.
            (
                1, 1, 0, 4, false, false, false,
                SegmentScripts {
                    imlaey: "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ".to_owned(),
                    uthmani: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ".to_owned(),
                    has_quran: true, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: false,
                    start_span: Some((1, 1, QuranWordIndex { imlaey: 0, uthmani: 0 })),
                    end_span: Some((1, 1, QuranWordIndex { imlaey: 4, uthmani: 4 })),
                },
            ),
            // Negative start crossing the sura boundary.
            (
                1, 1, -2, 6, false, false, false,
                SegmentScripts {
                    imlaey: "الْجِنَّةِ وَالنَّاسِ بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ".to_owned(),
                    uthmani: "ٱلْجِنَّةِ وَٱلنَّاسِ بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ".to_owned(),
                    has_quran: true, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: false,
                    start_span: Some((114, 6, QuranWordIndex { imlaey: 1, uthmani: 1 })),
                    end_span: Some((1, 1, QuranWordIndex { imlaey: 4, uthmani: 4 })),
                },
            ),
            // Larger negative window across three ayat.
            (
                1, 1, -10, 12, false, false, false,
                SegmentScripts {
                    imlaey: "الْوَسْوَاسِ الْخَنَّاسِ الَّذِي يُوَسْوِسُ فِي صُدُورِ النَّاسِ مِنَ الْجِنَّةِ وَالنَّاسِ بِسْمِ اللَّهِ".to_owned(),
                    uthmani: "ٱلْوَسْوَاسِ ٱلْخَنَّاسِ ٱلَّذِى يُوَسْوِسُ فِى صُدُورِ ٱلنَّاسِ مِنَ ٱلْجِنَّةِ وَٱلنَّاسِ بِسْمِ ٱللَّهِ".to_owned(),
                    has_quran: true, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: false,
                    start_span: Some((114, 4, QuranWordIndex { imlaey: 2, uthmani: 2 })),
                    end_span: Some((1, 1, QuranWordIndex { imlaey: 2, uthmani: 2 })),
                },
            ),
            // Merged-word window: two Imlaey words, one Uthmani word.
            (
                2, 21, 0, 5, false, false, false,
                SegmentScripts {
                    imlaey: "يَا أَيُّهَا النَّاسُ اعْبُدُوا رَبَّكُمُ".to_owned(),
                    uthmani: "يَـٰٓأَيُّهَا ٱلنَّاسُ ٱعْبُدُوا۟ رَبَّكُمُ".to_owned(),
                    has_quran: true, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: false,
                    start_span: Some((2, 21, QuranWordIndex { imlaey: 0, uthmani: 0 })),
                    end_span: Some((2, 21, QuranWordIndex { imlaey: 5, uthmani: 4 })),
                },
            ),
            // Positive window crossing into the next sura.
            (
                112, 4, 4, 20, false, false, false,
                SegmentScripts {
                    imlaey: "أَحَدٌ قُلْ أَعُوذُ بِرَبِّ الْفَلَقِ مِن شَرِّ مَا خَلَقَ وَمِن شَرِّ غَاسِقٍ إِذَا وَقَبَ وَمِن شَرِّ النَّفَّاثَاتِ فِي الْعُقَدِ وَمِن".to_owned(),
                    uthmani: "أَحَدٌۢ قُلْ أَعُوذُ بِرَبِّ ٱلْفَلَقِ مِن شَرِّ مَا خَلَقَ وَمِن شَرِّ غَاسِقٍ إِذَا وَقَبَ وَمِن شَرِّ ٱلنَّفَّـٰثَـٰتِ فِى ٱلْعُقَدِ وَمِن".to_owned(),
                    has_quran: true, has_istiaatha: false,
                    has_bismillah: false, has_sadaka: false,
                    start_span: Some((112, 4, QuranWordIndex { imlaey: 4, uthmani: 4 })),
                    end_span: Some((113, 5, QuranWordIndex { imlaey: 1, uthmani: 1 })),
                },
            ),
        ];
        for (sura, aya, start, window, istiaatha, bismillah, sadaka, expected) in cases {
            let aya = Aya::new(sura, aya).expect("vector aya must exist");
            assert_eq!(
                aya.get_by_imlaey_words(start, window, flags(istiaatha, bismillah, sadaka)),
                Ok(expected),
                "mismatch at {sura}:{aya} start {start} window {window}"
            );
        }
    }

    #[test]
    fn step_by_imlaey_words_matches_the_python_vectors() {
        let cases = [
            (1, 1, 0, 4, 1, 2, 0),
            (1, 1, -2, 6, 1, 2, 0),
            (1, 1, -10, 12, 1, 1, 2),
            (1, 1, -10, 20, 1, 4, 0),
        ];
        for (sura, aya, start, window, exp_sura, exp_aya, exp_word) in cases {
            let aya = Aya::new(sura, aya).expect("vector aya must exist");
            let out = aya.step_by_imlaey_words(start, window, EncodeFlags::default());
            assert_eq!(
                (out.sura_idx(), out.aya_idx(), out.start_imlaey_word_idx()),
                (exp_sura, exp_aya, exp_word),
                "mismatch at {sura}:{aya} start {start} window {window}"
            );
        }
    }
}
