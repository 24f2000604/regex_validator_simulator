//! Built-in example patterns, each with sample inputs (some match, some don't).
//!
//! Only the article's syntax is available — literals, `|`, `*`, `+`, `?`,
//! parentheses, plus `.` and `\x` — so character classes are spelled out as
//! alternations; the macros below keep those readable.

pub struct Preset {
    pub category: &'static str,
    pub name: &'static str,
    pub pattern: &'static str,
    pub inputs: &'static [&'static str],
}

/// One decimal digit.
macro_rules! d {
    () => {
        "(0|1|2|3|4|5|6|7|8|9)"
    };
}
/// One non-zero decimal digit.
macro_rules! nz {
    () => {
        "(1|2|3|4|5|6|7|8|9)"
    };
}
/// One lowercase hex digit.
macro_rules! hx {
    () => {
        "(0|1|2|3|4|5|6|7|8|9|a|b|c|d|e|f)"
    };
}
/// One lowercase letter.
macro_rules! lc {
    () => {
        "(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z)"
    };
}
/// One DNA base.
macro_rules! base {
    () => {
        "(A|C|G|T)"
    };
}

macro_rules! p {
    ($cat:expr, $name:expr, $re:expr, [$($i:expr),* $(,)?]) => {
        Preset { category: $cat, name: $name, pattern: $re, inputs: &[$($i),*] }
    };
}

const ART: &str = "From the article";
const LANG: &str = "Formal languages";
const BIN: &str = "Binary strings";
const PATH: &str = "Pathological (backtracking)";
const WORD: &str = "Words & text";
const NUM: &str = "Numbers & dates";
const CODE: &str = "Code & data formats";
const BIO: &str = "DNA & RNA";
const FUN: &str = "Games & misc";

pub const PRESETS: &[Preset] = &[
    // ── From the article ──
    p!(ART, "The article's example", "a(bb)+a", ["abba", "abbbba", "aa", "abbba", "abbbbbba"]),
    p!(ART, "a?ⁿaⁿ, n = 3", "a?a?a?aaa", ["aaa", "aaaa", "aaaaaa", "aa", "aaaaaaa"]),
    p!(ART, "Submatch example", "(a|ab)(c|bcd)(d*)", ["abcd", "acdd", "abbcd", "ac", "abd"]),
    p!(ART, "Nested star", "(a*)*b", ["b", "aaab", "aaa", "ba"]),
    p!(ART, "Dragon-book classic", "(a|b)*abb", ["abb", "aababb", "abab", "babbabb", ""]),
    p!(ART, "Any char with .", "h.llo", ["hello", "hallo", "h-llo", "hllo", "heello"]),

    // ── Formal languages over {a, b} ──
    p!(LANG, "Even number of a's", "(aa)*", ["", "aa", "aaaa", "a", "aaa"]),
    p!(LANG, "Odd number of a's", "a(aa)*", ["a", "aaa", "", "aa"]),
    p!(LANG, "Even a's (with b's)", "b*(ab*ab*)*", ["", "bab", "aabb", "abba", "a", "bab a"]),
    p!(LANG, "Exactly one a", "b*ab*", ["a", "bbabb", "b", "abab"]),
    p!(LANG, "At least two a's", "(a|b)*a(a|b)*a(a|b)*", ["aa", "babab", "ab", "bbb"]),
    p!(LANG, "Exactly two b's", "a*ba*ba*", ["bb", "abab", "aabaab", "b", "bbb"]),
    p!(LANG, "b count divisible by 3", "a*(ba*ba*ba*)*", ["", "bbb", "abababa", "bb", "bbbb"]),
    p!(LANG, "No two consecutive a's", "b*(abb*)*a?", ["", "ababa", "bab", "aa", "baab"]),
    p!(LANG, "Every a followed by b", "(b|ab)*", ["", "abab", "bbab", "aba", "aab"]),
    p!(LANG, "Doesn't contain ab", "b*a*", ["bbaa", "", "a", "ab", "aab"]),
    p!(LANG, "Contains aa or bb", "(a|b)*(aa|bb)(a|b)*", ["aa", "abba", "baab", "abab", "a"]),
    p!(LANG, "Contains bab", "(a|b)*bab(a|b)*", ["bab", "aababa", "abba", "bb"]),
    p!(LANG, "Contains abc", "(a|b|c)*abc(a|b|c)*", ["abc", "cabca", "acb", "ab"]),
    p!(LANG, "a's then b's", "a*b*", ["", "aabbb", "bb", "aba", "ba"]),
    p!(LANG, "a's, b's, then c's", "a+b+c+", ["abc", "aabbbc", "ac", "abcabc"]),
    p!(LANG, "x's, y's, z's (each optional)", "x*y*z*", ["", "xz", "xxyzz", "zx", "yx"]),
    p!(LANG, "(a*b*)* = (a|b)*", "(a*b*)*", ["", "abba", "bab", "abc"]),
    p!(LANG, "Repeated ab", "(ab)+", ["ab", "ababab", "", "aba", "ba"]),
    p!(LANG, "ab or ba pairs", "(ab|ba)*", ["", "abba", "baab", "aabb", "a"]),
    p!(LANG, "Same first and last letter", "a(a|b)*a|b(a|b)*b|a|b", ["a", "aba", "bab", "ab", "abba", "ba"]),
    p!(LANG, "Length divisible by 3", "((a|b)(a|b)(a|b))*", ["", "aba", "abbaba", "ab", "abab"]),
    p!(LANG, "Length 1 to 3", "(a|b)(a|b)?(a|b)?", ["a", "ab", "bab", "", "abab"]),
    p!(LANG, "2nd from last is a", "(a|b)*a(a|b)", ["ab", "bbaa", "ba", "abb"]),
    p!(LANG, "3rd from last is a", "(a|b)*a(a|b)(a|b)", ["abb", "baab", "bab", "ab"]),
    p!(LANG, "4th from last is a", "(a|b)*a(a|b)(a|b)(a|b)", ["abbb", "babab", "bbbb", "abb"]),
    p!(LANG, "a, b, c in order", ".*a.*b.*c.*", ["abc", "xaybzc", "cba", "acb"]),
    p!(LANG, "Balanced parens, depth ≤ 2", "(\\((\\(\\))*\\))*", ["", "()", "(())()", "(()())", "(()", ")("]),

    // ── Binary strings ──
    p!(BIN, "Multiple of 3", "(0|1(01*0)*1)*", ["0", "11", "110", "1001", "111", "100"]),
    p!(BIN, "Odd number", "(0|1)*1", ["1", "101", "0111", "10", ""]),
    p!(BIN, "Even, no leading zeros", "0|1(0|1)*0", ["0", "10", "1100", "01", "11"]),
    p!(BIN, "Exactly 4 bits", "(0|1)(0|1)(0|1)(0|1)", ["0000", "1011", "101", "10110"]),
    p!(BIN, "Even length", "((0|1)(0|1))*", ["", "01", "1100", "1", "101"]),
    p!(BIN, "No consecutive 1s", "(0|10)*1?", ["", "1010", "0100", "11", "0110"]),
    p!(BIN, "Alternating bits", "(01|10)*", ["", "0110", "1001", "0011"]),
    p!(BIN, "Exactly two 1s", "0*10*10*", ["11", "0101", "1000001", "1", "111"]),
    p!(BIN, "Palindromes, length ≤ 3", "0|1|00|11|000|010|101|111", ["0", "11", "010", "01", "110"]),
    p!(BIN, "Ends in 00 (multiple of 4)", "0|(0|1)*00", ["0", "100", "1100", "10", "1"]),
    p!(BIN, "Starts and ends with 1", "1|1(0|1)*1", ["1", "11", "1001", "10", "01"]),

    // ── Pathological for backtracking ──
    p!(PATH, "a?ⁿaⁿ, n = 5", "a?a?a?a?a?aaaaa", ["aaaaa", "aaaaaaaaaa", "aaaa"]),
    p!(PATH, "a?ⁿaⁿ, n = 10", "a?a?a?a?a?a?a?a?a?a?aaaaaaaaaa", ["aaaaaaaaaa", "aaaaaaaaaaaaaaaaaaaa", "aaaaaaaaa"]),
    p!(PATH, "a?ⁿaⁿ, n = 16", "a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?aaaaaaaaaaaaaaaa", ["aaaaaaaaaaaaaaaa", "aaaaaaaaaaaaaaa"]),
    p!(PATH, "Nested plus (a+)+b", "(a+)+b", ["ab", "aaaab", "aaaaaaaaaaaaaaaaaaaaaaaa"]),
    p!(PATH, "Overlapping alternation", "(a|a)*b", ["b", "aaab", "aaaaaaaaaaaaaaaaaaaa"]),
    p!(PATH, "Overlapping lengths", "(a|aa)*c", ["c", "aaac", "aaaaaaaaaaaaaaaaaaaaaaaaa"]),
    p!(PATH, "Nested star (a*)*c", "(a*)*c", ["c", "aaac", "aaaaaaaaaaaaaaaaaaaaaaaa"]),
    p!(PATH, "Doubled plus", "(x+x+)+y", ["xxy", "xxxxy", "xxxxxxxxxxxxxxxxxxxx"]),
    p!(PATH, "Many .* in a row", ".*a.*a.*a.*a.*a.*b", ["aaaaab", "xaxaxaxaxab", "aaaaaaaaaaaaaaaaaaaaaaaaa"]),
    p!(PATH, "Words separated by spaces", "(a+ ?)*b", ["a ab", "aa a b", "aaaaaaaaaaaaaaaaaaaaaaaa"]),

    // ── Words & text ──
    p!(WORD, "Color / colour", "colou?r", ["color", "colour", "colouur", "colr"]),
    p!(WORD, "Grey / gray", "gr(a|e)y", ["gray", "grey", "groy", "graey"]),
    p!(WORD, "Behavior / behaviour", "behaviou?r", ["behavior", "behaviour", "behaviur"]),
    p!(WORD, "Hello or world", "h.llo|w.rld", ["hello", "hallo", "world", "w0rld", "help"]),
    p!(WORD, "Pets", "cat|dog|bird|fish", ["cat", "fish", "cow", "cats"]),
    p!(WORD, "Walk + suffix", "walk(s|ed|ing|er)?", ["walk", "walked", "walking", "walkd"]),
    p!(WORD, "Happy forms", "(un)?happ(y|ier|iest|ily|iness)", ["happy", "unhappiness", "happily", "unhapp"]),
    p!(WORD, "Re-verbs", "(re)?(do|make|play|write)", ["do", "redo", "rewrite", "replays"]),
    p!(WORD, "Sing / sang / song / sung", "s(i|a|o|u)ng", ["sing", "sung", "seng", "sings"]),
    p!(WORD, "Rhymes with at", "(b|c|h|m|r|s)at", ["cat", "hat", "sat", "fat", "at"]),
    p!(WORD, "Weekdays", "(Mon|Tues|Wednes|Thurs|Fri|Satur|Sun)day", ["Monday", "Wednesday", "Wedday", "monday"]),
    p!(WORD, "Months", "Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec", ["Jan", "Sep", "Sept", "jan"]),
    p!(WORD, "Titles", "(Mr|Mrs|Ms|Dr)\\. .+", ["Mr. Smith", "Dr. Who", "Mr Smith", "Prof. X"]),
    p!(WORD, "The quick fox", "the (quick|slow) (brown|red) fox", ["the quick brown fox", "the slow red fox", "the quick fox"]),
    p!(WORD, "Laughter", "(ha)+!*|(he)+|lo+l", ["haha", "hahaha!!", "hehe", "loool", "hah"]),
    p!(WORD, "Yes or no", "y(es)?|no?", ["y", "yes", "n", "no", "ye", "nope"]),
    p!(WORD, "Only vowels", "(a|e|i|o|u)+", ["aeiou", "eau", "", "hai"]),
    p!(WORD, "Lowercase word", concat!(lc!(), "+"), ["hello", "regex", "Hello", "two words", ""]),
    p!(WORD, "Prefix + fix", "(pre|post|suf|in)fix", ["prefix", "infix", "suffix", "fix"]),
    p!(WORD, "Emphasis", "(very )*(good|bad)", ["good", "very very bad", "veryvery good"]),

    // ── Numbers & dates ──
    p!(NUM, "Unsigned integer", concat!(d!(), "+"), ["0", "42", "007", "", "4a"]),
    p!(NUM, "No leading zeros", concat!("0|", nz!(), d!(), "*"), ["0", "1234", "100", "007", "00"]),
    p!(NUM, "Signed integer", concat!("(\\+|-)?", d!(), "+"), ["42", "-7", "+100", "--1", "+"]),
    p!(NUM, "Decimal", concat!(d!(), "+(\\.", d!(), "+)?"), ["3", "3.14", "0.5", ".5", "3."]),
    p!(NUM, "Scientific notation", concat!(d!(), "+(\\.", d!(), "+)?((e|E)(\\+|-)?", d!(), "+)?"), ["6.02e23", "1E-9", "42", "1e", "e10"]),
    p!(NUM, "Even number", concat!(d!(), "*(0|2|4|6|8)"), ["4", "130", "1998", "7", "15"]),
    p!(NUM, "Multiple of 5", concat!(d!(), "*(0|5)"), ["5", "120", "1995", "12", "51"]),
    p!(NUM, "Percentage", concat!(d!(), "+(\\.", d!(), "+)?%"), ["50%", "99.9%", "100", "%"]),
    p!(NUM, "Dollar amount", concat!("\\$", d!(), "+(\\.", d!(), d!(), ")?"), ["$5", "$19.99", "$1.5", "19.99"]),
    p!(NUM, "24-hour time", concat!("((0|1)", d!(), "|2(0|1|2|3)):(0|1|2|3|4|5)", d!()), ["09:30", "23:59", "24:00", "12:60", "9:30"]),
    p!(NUM, "12-hour time", concat!("(", nz!(), "|1(0|1|2)):(0|1|2|3|4|5)", d!(), " (am|pm)"), ["9:30 am", "12:00 pm", "13:00 pm", "9:30"]),
    p!(NUM, "ISO date", concat!(d!(), d!(), d!(), d!(), "-(0", nz!(), "|1(0|1|2))-(0", nz!(), "|(1|2)", d!(), "|3(0|1))"), ["2026-11-27", "1999-01-31", "2026-13-01", "2026-02-32", "26-1-1"]),
    p!(NUM, "Year 1900–2099", concat!("(19|20)", d!(), d!()), ["1999", "2026", "1899", "21000"]),
    p!(NUM, "US ZIP code", concat!(d!(), d!(), d!(), d!(), d!(), "(-", d!(), d!(), d!(), d!(), ")?"), ["90210", "12345-6789", "1234", "12345-67"]),
    p!(NUM, "Phone number", concat!(d!(), d!(), d!(), "-", d!(), d!(), d!(), "-", d!(), d!(), d!(), d!()), ["555-123-4567", "5551234567", "555-1234"]),
    p!(NUM, "IPv4 (loose)", concat!(d!(), "+\\.", d!(), "+\\.", d!(), "+\\.", d!(), "+"), ["192.168.0.1", "10.0.0.255", "1.2.3", "a.b.c.d"]),
    p!(NUM, "Roman numeral", "M?M?M?(CM|CD|D?C?C?C?)(XC|XL|L?X?X?X?)(IX|IV|V?I?I?I?)", ["XIV", "MCMXCIX", "MMXXVI", "IIII", "VX"]),
    p!(NUM, "Hex literal", concat!("0x", hx!(), "+"), ["0xff", "0x1a2b", "0x", "ff", "0xg"]),
    p!(NUM, "Binary literal", "0b(0|1)+", ["0b1010", "0b0", "0b", "0b102"]),
    p!(NUM, "Octal literal", "0o(0|1|2|3|4|5|6|7)+", ["0o755", "0o0", "0o8", "755"]),

    // ── Code & data formats ──
    p!(CODE, "Keyword", "if|else|while|for|return|fn|let|mut", ["if", "return", "iff", "loop"]),
    p!(CODE, "Rust integer type", "(i|u)(8|16|32|64|128|size)", ["u8", "i64", "usize", "u7", "f32"]),
    p!(CODE, "Identifier (lowercase)", concat!("(", lc!(), "|_)(", lc!(), "|_|", d!(), ")*"), ["x", "my_var2", "_tmp", "2fast", "my-var"]),
    p!(CODE, "Operator", "\\+\\+|--|\\+=|-=|==|!=|<=|>=|&&|\\|\\|", ["++", "!=", "||", "=", "<>"]),
    p!(CODE, "Line comment", "//.*", ["// hi", "//", "/ not", "# no"]),
    p!(CODE, "Block comment", "/\\*.*\\*/", ["/* x */", "/**/", "/* open", "*/ /*"]),
    p!(CODE, "String literal", "\".*\"", ["\"hi\"", "\"\"", "\"open", "'single'"]),
    p!(CODE, "Boolean", "true|false", ["true", "false", "True", "yes"]),
    p!(CODE, "Null-ish", "null|nil|None|undefined", ["null", "None", "NULL", "void"]),
    p!(CODE, "HTML open tag", "<(b|i|u|p|em|div|span)>", ["<b>", "<div>", "<br>", "b"]),
    p!(CODE, "HTML close tag", "</(b|i|u|p|em|div|span)>", ["</p>", "</span>", "<p>", "</>"]),
    p!(CODE, "Source file name", concat!(lc!(), "+\\.(rs|toml|md|txt)"), ["main.rs", "cargo.toml", "main.py", ".rs"]),
    p!(CODE, "Unix path", concat!("(/", lc!(), "+)+/?"), ["/usr/bin", "/home/", "usr/bin", "//x"]),
    p!(CODE, "URL", "https?://(www\\.)?.+\\.(com|org|net)", ["https://example.com", "http://www.a.org", "ftp://a.com", "https://a.io"]),
    p!(CODE, "Email (loose)", ".+@.+\\..+", ["me@example.com", "a@b.c", "me@example", "@x.com"]),
    p!(CODE, "Semantic version", concat!(d!(), "+\\.", d!(), "+\\.", d!(), "+(-(alpha|beta|rc)(\\.", d!(), "+)?)?"), ["1.0.0", "0.2.2", "2.1.0-rc.1", "1.0", "1.0.0-dev"]),
    p!(CODE, "Short git hash", concat!(hx!(), hx!(), hx!(), hx!(), hx!(), hx!(), hx!()), ["a1b2c3d", "0000000", "a1b2c3", "a1b2c3g"]),
    p!(CODE, "CSS hex color", concat!("#", hx!(), hx!(), hx!(), "(", hx!(), hx!(), hx!(), ")?"), ["#fff", "#1e90ff", "#ffff", "fff"]),
    p!(CODE, "Variable declaration", "(let|var|const) .+ = .+;", ["let x = 1;", "const name = \"a\";", "let x = 1", "x = 1;"]),
    p!(CODE, "Key = value", concat!(lc!(), "+ ?= ?.+"), ["name = app", "debug=true", "= x", "Name = x"]),

    // ── DNA & RNA ──
    p!(BIO, "DNA sequence", concat!(base!(), "+"), ["GATTACA", "ACGT", "ACGU", ""]),
    p!(BIO, "RNA sequence", "(A|C|G|U)+", ["AUGC", "ACGT", "UUU"]),
    p!(BIO, "Open reading frame", concat!("ATG(", base!(), base!(), base!(), ")*(TAA|TAG|TGA)"), ["ATGTAA", "ATGAAACCCTGA", "ATGAATAA", "TAAATG"]),
    p!(BIO, "EcoRI site", concat!(base!(), "*GAATTC", base!(), "*"), ["GAATTC", "AAGAATTCGG", "GAATTG"]),
    p!(BIO, "CpG site", concat!(base!(), "*CG", base!(), "*"), ["ACGT", "CG", "GCAT"]),
    p!(BIO, "Poly-A tail", concat!(base!(), "*AAAAA*"), ["GCAAAA", "AAAAAAAA", "GCAAA", "AAAAT"]),

    // ── Games & misc ──
    p!(FUN, "Chess square", "(a|b|c|d|e|f|g|h)(1|2|3|4|5|6|7|8)", ["e4", "h8", "i1", "a9", "E4"]),
    p!(FUN, "Chess move", "(K|Q|R|B|N)?(a|b|c|d|e|f|g|h)?x?(a|b|c|d|e|f|g|h)(1|2|3|4|5|6|7|8)(\\+|#)?", ["e4", "Nf3", "Qxd7#", "exd5", "Z4", "Ke9"]),
    p!(FUN, "Castling", "O-O(-O)?", ["O-O", "O-O-O", "O-O-O-O", "0-0"]),
    p!(FUN, "Dice roll", concat!(nz!(), "?d(4|6|8|10|12|20|100)"), ["d20", "2d6", "3d7", "10d6"]),
    p!(FUN, "Playing card", "(A|2|3|4|5|6|7|8|9|10|J|Q|K)(♠|♥|♦|♣)", ["A♠", "10♥", "Q♣", "1♦", "K"]),
    p!(FUN, "Musical note", "(A|B|C|D|E|F|G)(#|b)?(0|1|2|3|4|5|6|7|8)?", ["C", "F#4", "Bb3", "H2", "C##"]),
    p!(FUN, "Morse code", "(\\.|-)+( (\\.|-)+)*", ["... --- ...", ".-", "..  .", ". _"]),
    p!(FUN, "Emoticon", "(:|;)-?(\\)|\\(|D|P)", [":)", ";-P", ":D", ":-|", "("]),
    p!(FUN, "Tic-tac-toe row", "(X|O|_)(X|O|_)(X|O|_)", ["XOX", "__O", "XO", "XYZ"]),
    p!(FUN, "Traffic light cycle", "(green yellow red ?)+", ["green yellow red", "green yellow red green yellow red ", "red green"]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{compile, sim::is_match};

    #[test]
    fn about_a_hundred() {
        assert!(PRESETS.len() >= 100, "{} presets", PRESETS.len());
    }

    #[test]
    fn every_preset_compiles_and_has_both_outcomes() {
        for p in PRESETS {
            let c = compile(p.pattern).unwrap_or_else(|e| panic!("{} ({}): {e}", p.name, p.pattern));
            let hits = p.inputs.iter().filter(|s| is_match(&c.nfa, s)).count();
            assert!(hits > 0, "{}: no sample matches", p.name);
            assert!(hits < p.inputs.len(), "{}: every sample matches", p.name);
        }
    }
}


/// Cross-checks the three matchers on every sample input: Thompson NFA
/// simulation, the lazily built DFA, and the backtracking matcher (which is
/// given a step budget, since some examples are built to defeat it).
#[cfg(test)]
mod agreement {
    use super::PRESETS;
    use crate::engine::{backtrack::backtrack, compile, dfa::Dfa, sim::is_match};

    #[test]
    fn nfa_dfa_and_backtracking_agree_on_every_sample() {
        let (mut total, mut bt_gave_up) = (0, 0);
        for p in PRESETS {
            let c = compile(p.pattern).unwrap();
            let mut dfa = Dfa::default();
            for s in p.inputs {
                total += 1;
                let nfa = is_match(&c.nfa, s);
                assert_eq!(dfa.run(&c.nfa, s).matched, nfa, "DFA disagrees: {} on {s:?}", p.name);
                let chars: Vec<char> = s.chars().collect();
                match backtrack(&c.nfa, &chars, 50_000_000).matched {
                    Some(bt) => assert_eq!(bt, nfa, "backtracking disagrees: {} on {s:?}", p.name),
                    None => bt_gave_up += 1,
                }
            }
        }
        println!("samples={total} backtracking_budget_exceeded={bt_gave_up}");
    }
}
