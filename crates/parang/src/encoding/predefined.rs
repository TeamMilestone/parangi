//! Predefined PDF character encodings.
//!
//! Ported from PDFBox: WinAnsiEncoding, StandardEncoding, MacRomanEncoding.

use std::sync::LazyLock;

use super::Encoding;

/// WinAnsiEncoding (PDF Reference Table D.1).
pub static WIN_ANSI: LazyLock<Encoding> = LazyLock::new(|| {
    let mut enc = Encoding::new("WinAnsiEncoding");
    for &(code, name) in WIN_ANSI_TABLE.iter() {
        enc.add(code, name);
    }
    // From PDF spec: In WinAnsiEncoding, all unused codes greater than 040 (32) map to bullet.
    for i in 33..=255u16 {
        if !enc.has_code(i) {
            enc.add(i, "bullet");
        }
    }
    enc
});

/// StandardEncoding (PDF Reference Table D.2).
pub static STANDARD: LazyLock<Encoding> = LazyLock::new(|| {
    let mut enc = Encoding::new("StandardEncoding");
    for &(code, name) in STANDARD_TABLE.iter() {
        enc.add(code, name);
    }
    enc
});

/// MacRomanEncoding (PDF Reference Table D.3).
pub static MAC_ROMAN: LazyLock<Encoding> = LazyLock::new(|| {
    let mut enc = Encoding::new("MacRomanEncoding");
    for &(code, name) in MAC_ROMAN_TABLE.iter() {
        enc.add(code, name);
    }
    enc
});

/// MacExpertEncoding.
pub static MAC_EXPERT: LazyLock<Encoding> = LazyLock::new(|| {
    let mut enc = Encoding::new("MacExpertEncoding");
    for &(code, name) in MAC_EXPERT_TABLE.iter() {
        enc.add(code, name);
    }
    enc
});

/// SymbolEncoding.
pub static SYMBOL: LazyLock<Encoding> = LazyLock::new(|| {
    let mut enc = Encoding::new("SymbolEncoding");
    for &(code, name) in SYMBOL_TABLE.iter() {
        enc.add(code, name);
    }
    enc
});

/// ZapfDingbatsEncoding.
pub static ZAPF_DINGBATS: LazyLock<Encoding> = LazyLock::new(|| {
    let mut enc = Encoding::new("ZapfDingbatsEncoding");
    for &(code, name) in ZAPF_DINGBATS_TABLE.iter() {
        enc.add(code, name);
    }
    enc
});

/// Get a predefined encoding by name.
pub fn get_encoding(name: &str) -> Option<&'static Encoding> {
    match name {
        "WinAnsiEncoding" => Some(&*WIN_ANSI),
        "StandardEncoding" => Some(&*STANDARD),
        "MacRomanEncoding" => Some(&*MAC_ROMAN),
        "MacExpertEncoding" => Some(&*MAC_EXPERT),
        "SymbolEncoding" => Some(&*SYMBOL),
        "ZapfDingbatsEncoding" => Some(&*ZAPF_DINGBATS),
        _ => None,
    }
}

// WinAnsiEncoding table — converted from Java octal to decimal.
// Source: PDFBox WinAnsiEncoding.java
static WIN_ANSI_TABLE: &[(u16, &str)] = &[
    (65, "A"), (198, "AE"), (193, "Aacute"), (194, "Acircumflex"),
    (196, "Adieresis"), (192, "Agrave"), (197, "Aring"), (195, "Atilde"),
    (66, "B"), (67, "C"), (199, "Ccedilla"), (68, "D"), (69, "E"),
    (201, "Eacute"), (202, "Ecircumflex"), (203, "Edieresis"), (200, "Egrave"),
    (208, "Eth"), (128, "Euro"), (70, "F"), (71, "G"), (72, "H"), (73, "I"),
    (205, "Iacute"), (206, "Icircumflex"), (207, "Idieresis"), (204, "Igrave"),
    (74, "J"), (75, "K"), (76, "L"), (77, "M"), (78, "N"), (209, "Ntilde"),
    (79, "O"), (140, "OE"), (211, "Oacute"), (212, "Ocircumflex"),
    (214, "Odieresis"), (210, "Ograve"), (216, "Oslash"), (213, "Otilde"),
    (80, "P"), (81, "Q"), (82, "R"), (83, "S"), (138, "Scaron"), (84, "T"),
    (222, "Thorn"), (85, "U"), (218, "Uacute"), (219, "Ucircumflex"),
    (220, "Udieresis"), (217, "Ugrave"), (86, "V"), (87, "W"), (88, "X"),
    (89, "Y"), (221, "Yacute"), (159, "Ydieresis"), (90, "Z"), (142, "Zcaron"),
    (97, "a"), (225, "aacute"), (226, "acircumflex"), (180, "acute"),
    (228, "adieresis"), (230, "ae"), (224, "agrave"), (38, "ampersand"),
    (229, "aring"), (94, "asciicircum"), (126, "asciitilde"), (42, "asterisk"),
    (64, "at"), (227, "atilde"), (98, "b"), (92, "backslash"), (124, "bar"),
    (123, "braceleft"), (125, "braceright"), (91, "bracketleft"),
    (93, "bracketright"), (166, "brokenbar"), (149, "bullet"), (99, "c"),
    (231, "ccedilla"), (184, "cedilla"), (162, "cent"), (136, "circumflex"),
    (58, "colon"), (44, "comma"), (169, "copyright"), (164, "currency"),
    (100, "d"), (134, "dagger"), (135, "daggerdbl"), (176, "degree"),
    (168, "dieresis"), (247, "divide"), (36, "dollar"), (101, "e"),
    (233, "eacute"), (234, "ecircumflex"), (235, "edieresis"), (232, "egrave"),
    (56, "eight"), (133, "ellipsis"), (151, "emdash"), (150, "endash"),
    (61, "equal"), (240, "eth"), (33, "exclam"), (161, "exclamdown"),
    (102, "f"), (53, "five"), (131, "florin"), (52, "four"), (103, "g"),
    (223, "germandbls"), (96, "grave"), (62, "greater"),
    (171, "guillemotleft"), (187, "guillemotright"), (139, "guilsinglleft"),
    (155, "guilsinglright"), (104, "h"), (45, "hyphen"), (105, "i"),
    (237, "iacute"), (238, "icircumflex"), (239, "idieresis"), (236, "igrave"),
    (106, "j"), (107, "k"), (108, "l"), (60, "less"), (172, "logicalnot"),
    (109, "m"), (175, "macron"), (181, "mu"), (215, "multiply"), (110, "n"),
    (57, "nine"), (241, "ntilde"), (35, "numbersign"), (111, "o"),
    (243, "oacute"), (244, "ocircumflex"), (246, "odieresis"), (156, "oe"),
    (242, "ograve"), (49, "one"), (189, "onehalf"), (188, "onequarter"),
    (185, "onesuperior"), (170, "ordfeminine"), (186, "ordmasculine"),
    (248, "oslash"), (245, "otilde"), (112, "p"), (182, "paragraph"),
    (40, "parenleft"), (41, "parenright"), (37, "percent"), (46, "period"),
    (183, "periodcentered"), (137, "perthousand"), (43, "plus"),
    (177, "plusminus"), (113, "q"), (63, "question"), (191, "questiondown"),
    (34, "quotedbl"), (132, "quotedblbase"), (147, "quotedblleft"),
    (148, "quotedblright"), (145, "quoteleft"), (146, "quoteright"),
    (130, "quotesinglbase"), (39, "quotesingle"), (114, "r"),
    (174, "registered"), (115, "s"), (154, "scaron"), (167, "section"),
    (59, "semicolon"), (55, "seven"), (54, "six"), (47, "slash"),
    (32, "space"), (163, "sterling"), (116, "t"), (254, "thorn"),
    (51, "three"), (190, "threequarters"), (179, "threesuperior"),
    (152, "tilde"), (153, "trademark"), (50, "two"), (178, "twosuperior"),
    (117, "u"), (250, "uacute"), (251, "ucircumflex"), (252, "udieresis"),
    (249, "ugrave"), (95, "underscore"), (118, "v"), (119, "w"), (120, "x"),
    (121, "y"), (253, "yacute"), (255, "ydieresis"), (165, "yen"),
    (122, "z"), (158, "zcaron"), (48, "zero"),
    // Additional from Appendix D
    (160, "nbspace"), (173, "sfthyphen"),
];

// StandardEncoding table — converted from Java octal to decimal.
// Source: PDFBox StandardEncoding.java
static STANDARD_TABLE: &[(u16, &str)] = &[
    (65, "A"), (225, "AE"), (66, "B"), (67, "C"), (68, "D"), (69, "E"),
    (70, "F"), (71, "G"), (72, "H"), (73, "I"), (74, "J"), (75, "K"),
    (76, "L"), (232, "Lslash"), (77, "M"), (78, "N"), (79, "O"),
    (234, "OE"), (216, "Oslash"), (80, "P"), (81, "Q"), (82, "R"),
    (83, "S"), (84, "T"), (85, "U"), (86, "V"), (87, "W"), (88, "X"),
    (89, "Y"), (90, "Z"), (97, "a"), (194, "acute"), (230, "ae"),
    (38, "ampersand"), (94, "asciicircum"), (126, "asciitilde"),
    (42, "asterisk"), (64, "at"), (98, "b"), (92, "backslash"),
    (124, "bar"), (123, "braceleft"), (125, "braceright"),
    (91, "bracketleft"), (93, "bracketright"), (198, "breve"),
    (183, "bullet"), (99, "c"), (207, "caron"), (203, "cedilla"),
    (162, "cent"), (195, "circumflex"), (58, "colon"), (44, "comma"),
    (168, "currency"), (100, "d"), (178, "dagger"), (179, "daggerdbl"),
    (200, "dieresis"), (36, "dollar"), (199, "dotaccent"),
    (245, "dotlessi"), (101, "e"), (56, "eight"), (188, "ellipsis"),
    (209, "emdash"), (208, "endash"), (61, "equal"), (33, "exclam"),
    (161, "exclamdown"), (102, "f"), (174, "fi"), (175, "fl"),
    (53, "five"), (166, "florin"), (52, "four"), (164, "fraction"),
    (103, "g"), (251, "germandbls"), (96, "grave"), (62, "greater"),
    (171, "guillemotleft"), (187, "guillemotright"),
    (172, "guilsinglleft"), (173, "guilsinglright"), (104, "h"),
    (45, "hyphen"), (105, "i"), (106, "j"), (107, "k"), (108, "l"),
    (60, "less"), (248, "lslash"), (109, "m"), (110, "n"), (57, "nine"),
    (35, "numbersign"), (111, "o"), (250, "oe"), (241, "onesuperior"),
    (227, "ordfeminine"), (235, "ordmasculine"), (248, "oslash"),
    (112, "p"), (182, "paragraph"), (40, "parenleft"),
    (41, "parenright"), (37, "percent"), (46, "period"),
    (180, "periodcentered"), (137, "perthousand"), (43, "plus"),
    (113, "q"), (63, "question"), (191, "questiondown"),
    (34, "quotedbl"), (185, "quotedblbase"), (170, "quotedblleft"),
    (186, "quotedblright"), (96, "quoteleft"), (39, "quoteright"),
    (184, "quotesinglbase"), (169, "quotesingle"), (114, "r"),
    (202, "ring"), (115, "s"), (167, "section"), (59, "semicolon"),
    (55, "seven"), (54, "six"), (47, "slash"), (32, "space"),
    (163, "sterling"), (116, "t"), (51, "three"), (196, "tilde"),
    (50, "two"), (117, "u"), (95, "underscore"), (118, "v"), (119, "w"),
    (120, "x"), (121, "y"), (165, "yen"), (122, "z"), (48, "zero"),
];

// MacRomanEncoding table — converted from Java octal to decimal.
// Source: PDFBox MacRomanEncoding.java
static MAC_ROMAN_TABLE: &[(u16, &str)] = &[
    (65, "A"), (174, "AE"), (231, "Aacute"), (229, "Acircumflex"),
    (128, "Adieresis"), (203, "Agrave"), (129, "Aring"), (204, "Atilde"),
    (66, "B"), (67, "C"), (130, "Ccedilla"), (68, "D"), (69, "E"),
    (131, "Eacute"), (230, "Ecircumflex"), (232, "Edieresis"),
    (233, "Egrave"), (70, "F"), (71, "G"), (72, "H"), (73, "I"),
    (234, "Iacute"), (235, "Icircumflex"), (236, "Idieresis"),
    (237, "Igrave"), (74, "J"), (75, "K"), (76, "L"), (77, "M"),
    (78, "N"), (132, "Ntilde"), (79, "O"), (206, "OE"),
    (238, "Oacute"), (239, "Ocircumflex"), (133, "Odieresis"),
    (241, "Ograve"), (175, "Oslash"), (205, "Otilde"), (80, "P"),
    (81, "Q"), (82, "R"), (83, "S"), (84, "T"), (85, "U"),
    (242, "Uacute"), (243, "Ucircumflex"), (134, "Udieresis"),
    (244, "Ugrave"), (86, "V"), (87, "W"), (88, "X"), (89, "Y"),
    (217, "Ydieresis"), (90, "Z"), (97, "a"), (135, "aacute"),
    (137, "acircumflex"), (171, "acute"), (138, "adieresis"),
    (190, "ae"), (136, "agrave"), (38, "ampersand"), (140, "aring"),
    (94, "asciicircum"), (126, "asciitilde"), (42, "asterisk"),
    (64, "at"), (139, "atilde"), (98, "b"), (92, "backslash"),
    (124, "bar"), (123, "braceleft"), (125, "braceright"),
    (91, "bracketleft"), (93, "bracketright"), (249, "breve"),
    (165, "bullet"), (99, "c"), (255, "caron"), (141, "ccedilla"),
    (252, "cedilla"), (162, "cent"), (246, "circumflex"), (58, "colon"),
    (44, "comma"), (169, "copyright"), (219, "currency"), (100, "d"),
    (160, "dagger"), (224, "daggerdbl"), (161, "degree"),
    (172, "dieresis"), (214, "divide"), (36, "dollar"),
    (250, "dotaccent"), (245, "dotlessi"), (101, "e"),
    (142, "eacute"), (144, "ecircumflex"), (143, "edieresis"),
    (56, "eight"), (201, "ellipsis"), (209, "emdash"), (208, "endash"),
    (61, "equal"), (33, "exclam"), (193, "exclamdown"), (102, "f"),
    (222, "fi"), (53, "five"), (223, "fl"), (196, "florin"),
    (52, "four"), (103, "g"), (167, "germandbls"), (96, "grave"),
    (62, "greater"), (199, "guillemotleft"), (200, "guillemotright"),
    (220, "guilsinglleft"), (221, "guilsinglright"), (104, "h"),
    (45, "hyphen"), (105, "i"), (146, "iacute"), (148, "icircumflex"),
    (149, "idieresis"), (145, "igrave"), (106, "j"), (107, "k"),
    (108, "l"), (60, "less"), (194, "logicalnot"), (109, "m"),
    (248, "macron"), (181, "mu"), (110, "n"), (57, "nine"),
    (150, "ntilde"), (35, "numbersign"), (111, "o"), (151, "oacute"),
    (153, "ocircumflex"), (154, "odieresis"), (207, "oe"),
    (152, "ograve"), (49, "one"), (187, "ordfeminine"),
    (188, "ordmasculine"), (191, "oslash"), (155, "otilde"), (112, "p"),
    (166, "paragraph"), (40, "parenleft"), (41, "parenright"),
    (37, "percent"), (46, "period"), (225, "periodcentered"),
    (228, "perthousand"), (43, "plus"), (177, "plusminus"), (113, "q"),
    (63, "question"), (192, "questiondown"), (34, "quotedbl"),
    (227, "quotedblbase"), (210, "quotedblleft"),
    (211, "quotedblright"), (212, "quoteleft"), (213, "quoteright"),
    (226, "quotesinglbase"), (39, "quotesingle"), (114, "r"),
    (168, "registered"), (251, "ring"), (115, "s"), (164, "section"),
    (59, "semicolon"), (55, "seven"), (54, "six"), (47, "slash"),
    (32, "space"), (163, "sterling"), (116, "t"), (51, "three"),
    (247, "tilde"), (50, "two"), (117, "u"), (156, "uacute"),
    (158, "ucircumflex"), (159, "udieresis"), (157, "ugrave"),
    (95, "underscore"), (118, "v"), (119, "w"), (120, "x"), (121, "y"),
    (216, "ydieresis"), (180, "yen"), (122, "z"), (48, "zero"),
    // Additional
    (202, "nbspace"),
];

// MacExpertEncoding (subset - most commonly referenced entries)
static MAC_EXPERT_TABLE: &[(u16, &str)] = &[
    (32, "space"), (33, "exclamsmall"), (34, "Hungarumlautsmall"),
    (36, "dollaroldstyle"), (38, "ampersandsmall"),
    (39, "Acutesmall"), (40, "parenleftsuperior"),
    (41, "parenrightsuperior"), (42, "twodotenleader"),
    (44, "commainferior"), (45, "hypheninferior"),
    (46, "periodcentered"), (47, "periodinferior"),
    (48, "zerooldstyle"), (49, "oneoldstyle"), (50, "twooldstyle"),
    (51, "threeoldstyle"), (52, "fouroldstyle"), (53, "fiveoldstyle"),
    (54, "sixoldstyle"), (55, "sevenoldstyle"), (56, "eightoldstyle"),
    (57, "nineoldstyle"), (58, "colonmonetary"), (59, "commasuperior"),
    (61, "threequartersemdash"), (63, "questionsmall"),
    (105, "isuperior"), (108, "lsuperior"), (110, "nsuperior"),
    (114, "rsuperior"), (115, "ssuperior"), (116, "tsuperior"),
    (156, "centoldstyle"), (251, "Gravesmall"), (253, "Dieresissmall"),
    (255, "Cedillasmall"),
];

// SymbolEncoding (subset - Greek letters and math symbols)
static SYMBOL_TABLE: &[(u16, &str)] = &[
    (32, "space"), (33, "exclam"), (34, "universal"), (35, "numbersign"),
    (36, "existential"), (37, "percent"), (38, "ampersand"),
    (39, "suchthat"), (40, "parenleft"), (41, "parenright"),
    (42, "asteriskmath"), (43, "plus"), (44, "comma"), (45, "minus"),
    (46, "period"), (47, "slash"), (48, "zero"), (49, "one"),
    (50, "two"), (51, "three"), (52, "four"), (53, "five"),
    (54, "six"), (55, "seven"), (56, "eight"), (57, "nine"),
    (58, "colon"), (59, "semicolon"), (60, "less"), (61, "equal"),
    (62, "greater"), (63, "question"), (64, "congruent"),
    (65, "Alpha"), (66, "Beta"), (67, "Chi"), (68, "Delta"),
    (69, "Epsilon"), (70, "Phi"), (71, "Gamma"), (72, "Eta"),
    (73, "Iota"), (74, "theta1"), (75, "Kappa"), (76, "Lambda"),
    (77, "Mu"), (78, "Nu"), (79, "Omicron"), (80, "Pi"), (81, "Theta"),
    (82, "Rho"), (83, "Sigma"), (84, "Tau"), (85, "Upsilon"),
    (86, "sigma1"), (87, "Omega"), (88, "Xi"), (89, "Psi"),
    (90, "Zeta"), (91, "bracketleft"), (92, "therefore"),
    (93, "bracketright"), (94, "perpendicular"), (95, "underscore"),
    (97, "alpha"), (98, "beta"), (99, "chi"), (100, "delta"),
    (101, "epsilon"), (102, "phi"), (103, "gamma"), (104, "eta"),
    (105, "iota"), (106, "phi1"), (107, "kappa"), (108, "lambda"),
    (109, "mu"), (110, "nu"), (111, "omicron"), (112, "pi"),
    (113, "theta"), (114, "rho"), (115, "sigma"), (116, "tau"),
    (117, "upsilon"), (118, "omega1"), (119, "omega"), (120, "xi"),
    (121, "psi"), (122, "zeta"), (123, "braceleft"), (124, "bar"),
    (125, "braceright"), (126, "similar"),
    (160, "Euro"), (163, "lessequal"), (165, "infinity"),
    (170, "spade"), (171, "heart"), (172, "diamond"), (173, "club"),
    (176, "degree"), (177, "plusminus"),
    (180, "multiply"), (181, "proportional"),
    (182, "partialdiff"), (183, "bullet"),
    (184, "divide"), (185, "notequal"), (186, "equivalence"),
    (187, "approxequal"),
    (191, "carriagereturn"), (192, "aleph"),
    (215, "circlemultiply"), (216, "circleplus"),
    (229, "summation"), (242, "integral"),
];

// ZapfDingbatsEncoding (subset)
static ZAPF_DINGBATS_TABLE: &[(u16, &str)] = &[
    (32, "space"), (33, "a1"), (34, "a2"), (35, "a202"), (36, "a3"),
    (37, "a4"), (38, "a5"), (39, "a119"), (40, "a118"), (41, "a117"),
    (42, "a11"), (43, "a12"), (44, "a13"), (45, "a14"), (46, "a15"),
    (47, "a16"), (48, "a105"), (49, "a17"), (50, "a18"), (51, "a19"),
    (52, "a20"), (53, "a21"), (54, "a22"), (55, "a23"), (56, "a24"),
    (57, "a25"), (58, "a26"), (59, "a27"), (60, "a28"), (61, "a6"),
    (62, "a7"), (63, "a8"), (64, "a9"), (65, "a10"), (66, "a29"),
    (67, "a30"), (68, "a31"), (69, "a32"), (70, "a33"), (71, "a34"),
    (72, "a35"), (73, "a36"), (74, "a37"), (75, "a38"), (76, "a39"),
    (77, "a40"), (78, "a41"), (79, "a42"), (80, "a43"), (81, "a44"),
    (82, "a45"), (83, "a46"), (84, "a47"), (85, "a48"), (86, "a49"),
    (87, "a50"), (88, "a51"), (89, "a52"), (90, "a53"), (91, "a54"),
    (92, "a55"), (93, "a56"), (94, "a57"), (95, "a58"), (96, "a59"),
    (97, "a60"), (98, "a61"), (99, "a62"), (100, "a63"), (101, "a64"),
    (102, "a65"), (103, "a66"), (104, "a67"), (105, "a68"),
    (106, "a69"), (107, "a70"), (108, "a71"), (109, "a72"),
    (110, "a73"), (111, "a74"), (112, "a203"), (113, "a75"),
    (114, "a204"), (115, "a76"), (116, "a77"), (117, "a78"),
    (118, "a79"), (119, "a81"), (120, "a82"), (121, "a83"),
    (122, "a84"), (123, "a97"), (124, "a98"), (125, "a99"),
    (126, "a100"),
    (161, "a101"), (162, "a102"), (163, "a103"), (164, "a104"),
    (165, "a106"), (166, "a107"), (167, "a108"), (168, "a112"),
    (169, "a111"), (170, "a110"), (171, "a109"), (172, "a120"),
    (173, "a121"), (174, "a122"), (175, "a123"), (176, "a124"),
    (177, "a125"), (178, "a126"), (179, "a127"), (180, "a128"),
    (181, "a129"), (182, "a130"), (183, "a131"), (184, "a132"),
    (185, "a133"), (186, "a134"), (187, "a135"), (188, "a136"),
    (189, "a137"), (190, "a138"), (191, "a139"), (192, "a140"),
    (193, "a141"), (194, "a142"), (195, "a143"), (196, "a144"),
    (197, "a145"), (198, "a146"), (199, "a147"), (200, "a148"),
    (201, "a149"), (202, "a150"), (203, "a151"), (204, "a152"),
    (205, "a153"), (206, "a154"), (207, "a155"), (208, "a156"),
    (209, "a157"), (210, "a158"), (211, "a159"), (212, "a160"),
    (213, "a161"), (214, "a163"), (215, "a164"), (216, "a196"),
    (217, "a165"), (218, "a192"), (219, "a166"), (220, "a167"),
    (221, "a168"), (222, "a169"), (223, "a170"), (224, "a171"),
    (225, "a172"), (226, "a173"), (227, "a162"), (228, "a174"),
    (229, "a175"), (230, "a176"), (231, "a177"), (232, "a178"),
    (233, "a179"), (234, "a193"), (235, "a180"), (236, "a199"),
    (237, "a181"), (238, "a200"), (239, "a182"),
    (241, "a201"), (242, "a183"), (243, "a184"), (244, "a197"),
    (245, "a185"), (246, "a194"), (247, "a198"), (248, "a186"),
    (249, "a195"), (250, "a187"), (251, "a188"), (252, "a189"),
    (253, "a190"), (254, "a191"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win_ansi() {
        let enc = &*WIN_ANSI;
        assert_eq!(enc.get_name(65), Some("A"));
        assert_eq!(enc.get_name(32), Some("space"));
        assert_eq!(enc.get_name(128), Some("Euro"));
        // Unmapped codes > 32 should map to bullet
        assert_eq!(enc.get_name(129), Some("bullet"));
    }

    #[test]
    fn test_standard() {
        let enc = &*STANDARD;
        assert_eq!(enc.get_name(65), Some("A"));
        assert_eq!(enc.get_name(32), Some("space"));
        // Code 128 is not mapped in StandardEncoding
        assert_eq!(enc.get_name(128), None);
    }

    #[test]
    fn test_mac_roman() {
        let enc = &*MAC_ROMAN;
        assert_eq!(enc.get_name(65), Some("A"));
        assert_eq!(enc.get_name(128), Some("Adieresis"));
    }

    #[test]
    fn test_get_encoding() {
        assert!(get_encoding("WinAnsiEncoding").is_some());
        assert!(get_encoding("StandardEncoding").is_some());
        assert!(get_encoding("MacRomanEncoding").is_some());
        assert!(get_encoding("Unknown").is_none());
    }
}
