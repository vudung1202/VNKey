//! Differential test: `vnkey_engine::convert_util` vs the vendored C++
//! `convertUtil`, driven with identical `(source, from_code, to_code, case
//! options)` and compared byte-for-byte.
//!
//! Two passes:
//! - a deterministic golden matrix over all 5x5 table pairs x each case
//!   option on a fixed multi-line Vietnamese sample, and
//! - a proptest pass over random ASCII+Vietnamese text with random table
//!   pairs and options.
//!
//! Because this is a *differential* comparison, the "from_code" need not match
//! the sample's true encoding: both engines receive the same UTF-8 bytes and
//! options, so they must agree regardless. NUL is excluded from generated
//! input because C++ `utf8ToWideString` truncates at `.c_str()`'s first NUL
//! (a constraint `convert_util` mirrors, but which would make random NUL
//! placement a no-signal axis).

use openkey_oracle::{OkConvertOpts, OkOptions, Oracle};
use proptest::prelude::*;
use vnkey_engine::{CodeTable, ConvertOptions, convert_util};

const TABLES: [CodeTable; 5] = [
    CodeTable::Unicode,
    CodeTable::Tcvn3,
    CodeTable::VniWindows,
    CodeTable::UnicodeCompound,
    CodeTable::Cp1258,
];

/// The six mutually-exercised case/mark axes (each applied singly, mirroring
/// how the OpenKey UI offers them as radio-ish toggles).
#[derive(Debug, Clone, Copy)]
enum CaseOpt {
    None,
    RemoveMark,
    AllCaps,
    AllNonCaps,
    CapsFirstLetter,
    CapsEachWord,
}

fn to_convert_options(from: CodeTable, to: CodeTable, case: CaseOpt) -> ConvertOptions {
    let mut o = ConvertOptions {
        from_code: from,
        to_code: to,
        ..ConvertOptions::default()
    };
    match case {
        CaseOpt::None => {}
        CaseOpt::RemoveMark => o.remove_mark = true,
        CaseOpt::AllCaps => o.to_all_caps = true,
        CaseOpt::AllNonCaps => o.to_all_non_caps = true,
        CaseOpt::CapsFirstLetter => o.to_caps_first_letter = true,
        CaseOpt::CapsEachWord => o.to_caps_each_word = true,
    }
    o
}

fn to_ok_convert(o: &ConvertOptions) -> OkConvertOpts {
    OkConvertOpts {
        from_code: o.from_code as i32,
        to_code: o.to_code as i32,
        remove_mark: o.remove_mark as i32,
        to_all_caps: o.to_all_caps as i32,
        to_all_non_caps: o.to_all_non_caps as i32,
        to_caps_first_letter: o.to_caps_first_letter as i32,
        to_caps_each_word: o.to_caps_each_word as i32,
    }
}

/// One `convert_util` comparison: Rust `String` bytes vs the oracle's raw
/// output bytes.
///
/// When the oracle *throws* (a lone-surrogate `to_bytes` error -- an OpenKey
/// crash bug: `convertUtil`'s single-unit emit branch pushes a whole VNI/CP1258
/// 2-byte cell that lands in 0xD800..0xDFFF; reachable when a string's last
/// matched char is such a cell), the case is excluded per the "C++ UB/crash ->
/// exclude" divergence rule. `vnkey_engine::convert_util` does not crash on
/// those inputs (its `String::from_utf16` falls back to replacement chars), so
/// its behavior there is intentionally unconstrained by the oracle.
fn compare_convert(src: &str, o: &ConvertOptions) -> Result<(), TestCaseError> {
    let ok_opts = OkOptions::default();
    let mut session = Oracle::lock(&ok_opts);
    let Some(oracle) = session.convert_util(src, &to_ok_convert(o)) else {
        return Ok(()); // oracle crashed on this input -> excluded
    };
    let rust = convert_util(src, o);
    prop_assert_eq!(
        rust.as_bytes(),
        oracle.as_slice(),
        "convert mismatch: src={:?} opts={:?}\n  rust={:?}\n  oracle={:?}",
        src,
        o,
        rust,
        String::from_utf8_lossy(&oracle)
    );
    Ok(())
}

const GOLDEN_SAMPLE: &str = "Tiếng Việt rất đẹp!\nHọc lập trình Rust.\nĐây là dòng thứ ba? Vâng.";

#[test]
fn convert_golden_matrix_all_tables_all_case_options() {
    let cases = [
        CaseOpt::None,
        CaseOpt::RemoveMark,
        CaseOpt::AllCaps,
        CaseOpt::AllNonCaps,
        CaseOpt::CapsFirstLetter,
        CaseOpt::CapsEachWord,
    ];
    for &from in &TABLES {
        for &to in &TABLES {
            for &case in &cases {
                let o = to_convert_options(from, to, case);
                compare_convert(GOLDEN_SAMPLE, &o)
                    .unwrap_or_else(|e| panic!("golden {from:?}->{to:?} {case:?}: {e:?}"));
            }
        }
    }
}

#[test]
fn convert_golden_multiline_sentence_casing() {
    // Focused check of the sentence/word casing state machine (break codes
    // '.', '?', '!' + spaces/newlines).
    let text = "chao ban. toi ten la nam! ban ten gi? rat vui.";
    for case in [CaseOpt::CapsFirstLetter, CaseOpt::CapsEachWord] {
        let o = to_convert_options(CodeTable::Unicode, CodeTable::Unicode, case);
        compare_convert(text, &o).unwrap_or_else(|e| panic!("{case:?}: {e:?}"));
    }
}

/// Random ASCII + Vietnamese characters (no NUL). A small mix of accented
/// letters, break codes, spaces and newlines keeps the sentence-case and
/// tone/mark table paths well exercised.
fn text_strategy() -> impl Strategy<Value = String> {
    let chars = "aAeEoOuUiIndNDĐ áàảãạ ÂÊÔ ưởịệ .?! ,\n0123";
    let pool: Vec<char> = chars.chars().filter(|&c| c != ' ' || true).collect();
    prop::collection::vec(0usize..pool.len(), 0..=40)
        .prop_map(move |idxs| idxs.into_iter().map(|i| pool[i]).collect())
}

fn table_strategy() -> impl Strategy<Value = CodeTable> {
    (0usize..5).prop_map(|i| TABLES[i])
}

fn case_strategy() -> impl Strategy<Value = CaseOpt> {
    prop_oneof![
        Just(CaseOpt::None),
        Just(CaseOpt::RemoveMark),
        Just(CaseOpt::AllCaps),
        Just(CaseOpt::AllNonCaps),
        Just(CaseOpt::CapsFirstLetter),
        Just(CaseOpt::CapsEachWord),
    ]
}

fn config(default_cases: u32) -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default_cases);
    ProptestConfig::with_cases(cases)
}

proptest! {
    #![proptest_config(config(5000))]

    #[test]
    fn convert_differential_random_text(
        src in text_strategy(),
        from in table_strategy(),
        to in table_strategy(),
        case in case_strategy(),
    ) {
        let o = to_convert_options(from, to, case);
        compare_convert(&src, &o)?;
    }
}
