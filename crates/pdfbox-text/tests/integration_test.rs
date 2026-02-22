//! Integration tests using real PDF files.

use std::path::Path;
use pdfbox_text::PdfDocument;
use pdfbox_text::cos_helpers::DocumentExt;
use pdfbox_text::encoding::cmap_parser;
use pdfbox_text::font::PdfFont;
use pdfbox_text::stream::engine::StreamEngine;

const SAMPLE_KOREAN_PDF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/sample_korean.pdf"
);

#[test]
fn test_open_real_pdf() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        eprintln!("Skipping test: sample PDF not found at {}", path.display());
        return;
    }
    let doc = PdfDocument::open(path).expect("should open sample Korean PDF");
    assert!(doc.page_count() > 0, "PDF should have at least one page");
    println!("Page count: {}", doc.page_count());
}

#[test]
fn test_page_properties() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let page = doc.page(0).expect("should get first page");

    let media_box = page.media_box().expect("should have media box");
    println!("Media box: {:?}", media_box);
    // Typical A4/Letter page should have reasonable dimensions
    assert!(media_box[2] > 100.0, "page width should be > 100pt");
    assert!(media_box[3] > 100.0, "page height should be > 100pt");

    let rotation = page.rotation().expect("should get rotation");
    assert!(
        rotation == 0 || rotation == 90 || rotation == 180 || rotation == 270,
        "rotation should be 0/90/180/270, got {}",
        rotation
    );
}

#[test]
fn test_page_resources() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let page = doc.page(0).unwrap();

    let resources = page.resources().expect("should get resources");
    assert!(resources.is_some(), "page should have resources");

    let res = resources.unwrap();
    let font_names = res.font_names().expect("should get font names");
    println!("Font names on page 0: {:?}", font_names.iter().map(|n| String::from_utf8_lossy(n).to_string()).collect::<Vec<_>>());
    assert!(!font_names.is_empty(), "page should have at least one font");
}

#[test]
fn test_content_stream_decompression() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let page = doc.page(0).unwrap();

    let content_bytes = page.content_bytes().expect("should get content bytes");
    assert!(!content_bytes.is_empty(), "content stream should not be empty");
    println!("Content stream size: {} bytes", content_bytes.len());

    // Content stream should contain PDF operators
    let content_str = String::from_utf8_lossy(&content_bytes);
    // Common operators in text-containing PDFs
    let has_text_ops = content_str.contains("BT") || content_str.contains("Tj") || content_str.contains("TJ");
    assert!(has_text_ops, "content stream should contain text operators");
}

#[test]
fn test_basic_text_extraction() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let page = doc.page(0).unwrap();

    let text = page.extract_text_basic().expect("basic text extraction should work");
    // Should produce some output (even if garbled due to no font decoding yet)
    assert!(!text.is_empty(), "should extract some text from Korean PDF");
    println!("Extracted text length: {} chars", text.len());
}

#[test]
fn test_all_pages_content_streams() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).expect(&format!("should get page {}", i));
        let content = page.content_bytes();
        assert!(content.is_ok(), "page {} content_bytes should not error", i);
        println!("Page {} content: {} bytes", i, content.unwrap().len());
    }
}

#[test]
fn test_font_dict_access() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let page = doc.page(0).unwrap();
    let resources = page.resources().unwrap().unwrap();

    let font_names = resources.font_names().unwrap();
    for name in &font_names {
        let font_dict = resources.get_font_dict(name).expect("should get font dict");
        assert!(font_dict.is_some(), "font {:?} should exist", String::from_utf8_lossy(name));

        let (dict, _oid) = font_dict.unwrap();
        // Every font dict should have a Type entry
        let type_name = dict.get(b"Type");
        if let Ok(obj) = type_name {
            if let lopdf::Object::Name(n) = obj {
                assert_eq!(n, b"Font", "Type should be Font");
            }
        }

        // Check Subtype
        let subtype = dict.get(b"Subtype");
        assert!(subtype.is_ok(), "font should have Subtype");
        if let Ok(lopdf::Object::Name(n)) = subtype {
            let subtype_str = String::from_utf8_lossy(n);
            println!("Font {:?}: Subtype={}", String::from_utf8_lossy(name), subtype_str);
            assert!(
                ["Type0", "Type1", "TrueType", "Type3", "CIDFontType0", "CIDFontType2", "MMType1"]
                    .iter()
                    .any(|s| subtype_str == *s),
                "unexpected font subtype: {}",
                subtype_str
            );
        }
    }
}

#[test]
fn test_stream_engine_real_pdf() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let page = doc.page(0).unwrap();
    let content_bytes = page.content_bytes().unwrap();

    // Load resources (fonts + xobjects)
    let resources = page.resources().unwrap().unwrap();
    let rotation = page.rotation().unwrap_or(0) as i32;
    let media_box = page.media_box().unwrap();

    let mut engine = StreamEngine::new(doc.inner_arc());
    engine.set_page_info(rotation, media_box[2], media_box[3]);
    engine.load_resources(resources.dictionary());
    engine.process_content(&content_bytes).expect("stream engine should process content");

    let positions = engine.text_positions();
    assert!(!positions.is_empty(), "should extract text positions from real PDF");
    println!("Text positions extracted: {}", positions.len());

    // Verify positions have Unicode text
    let with_unicode = positions.iter().filter(|p| !p.unicode.is_empty()).count();
    println!("Positions with unicode: {}/{}", with_unicode, positions.len());
    assert!(with_unicode > 0, "at least some positions should have unicode text");

    // Print first few text positions for inspection
    for (i, tp) in positions.iter().take(20).enumerate() {
        println!(
            "  [{}] '{}' at ({:.1}, {:.1}) w={:.2} fs={:.0}",
            i, tp.unicode, tp.x(), tp.y(), tp.individual_width, tp.font_size
        );
    }
}

#[test]
fn test_tounicode_cmap_parsing() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let inner = doc.inner_arc();
    let page = doc.page(0).unwrap();
    let resources = page.resources().unwrap().unwrap();

    let font_names = resources.font_names().unwrap();
    let mut found_tounicode = false;

    for name in &font_names {
        if let Some((font_dict, _oid)) = resources.get_font_dict(name).unwrap() {
            // Check for ToUnicode stream
            if let Ok(to_unicode_obj) = font_dict.get(b"ToUnicode") {
                let stream_data = inner.get_stream_data(to_unicode_obj);
                if let Ok(data) = stream_data {
                    let cmap = cmap_parser::parse_cmap(&data);
                    println!(
                        "Font {:?}: ToUnicode CMap has {} Unicode mappings",
                        String::from_utf8_lossy(name),
                        cmap.unicode_mapping_count()
                    );
                    if cmap.unicode_mapping_count() > 0 {
                        found_tounicode = true;
                        // Try to look up some common code values
                        let ranges = cmap.codespace_ranges();
                        println!("  Codespace ranges: {}", ranges.len());
                        for r in ranges {
                            println!(
                                "    length={}, start={:?}, end={:?}",
                                r.code_length, r.start, r.end
                            );
                        }
                    }
                }
            }
        }
    }

    assert!(found_tounicode, "should find at least one font with a ToUnicode CMap");
}

#[test]
fn test_font_creation_and_decode() {
    let path = Path::new(SAMPLE_KOREAN_PDF);
    if !path.exists() {
        return;
    }
    let doc = PdfDocument::open(path).unwrap();
    let inner = doc.inner_arc();
    let page = doc.page(0).unwrap();
    let resources = page.resources().unwrap().unwrap();

    let font_names = resources.font_names().unwrap();
    let mut created_fonts = 0;
    let mut decoded_chars = 0;

    for name in &font_names {
        if let Some((font_dict, oid)) = resources.get_font_dict(name).unwrap() {
            match PdfFont::from_dict(&inner, font_dict, oid) {
                Ok(font) => {
                    created_fonts += 1;
                    let font_name = String::from_utf8_lossy(name);

                    // Try decoding some common codes
                    for code in 32..=126 {
                        if let Some(unicode) = font.to_unicode(code) {
                            decoded_chars += 1;
                            if code == 65 {
                                println!("Font {:?}: code 65 → {:?}", font_name, unicode);
                            }
                        }
                    }
                    println!(
                        "Font {:?}: decoded {} chars in 32-126 range, width(65)={}",
                        font_name,
                        (32..=126).filter(|&c| font.to_unicode(c).is_some()).count(),
                        font.get_width(65)
                    );
                }
                Err(e) => {
                    println!("Font {:?}: creation error: {}", String::from_utf8_lossy(name), e);
                }
            }
        }
    }

    assert!(created_fonts > 0, "should create at least one font");
    println!("Created {} fonts, decoded {} chars total", created_fonts, decoded_chars);
}
