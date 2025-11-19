//! Create large test fixtures for performance testing
//! 
//! This example creates:
//! - large_sample.xlsx with 10,000 cells
//! - large_sample.pptx with 100 slides

use std::fs::File;
use std::io::Write;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating large test fixtures for performance testing...");
    
    create_large_xlsx()?;
    create_large_pptx()?;
    
    println!("All large test fixtures created successfully!");
    Ok(())
}

fn create_large_xlsx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating large_sample.xlsx with 10,000 cells...");
    
    let file = File::create("test_data/document/large_sample.xlsx")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#)?;
    
    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#)?;
    
    // docProps/core.xml
    zip.start_file("docProps/core.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<dc:creator>Performance Test</dc:creator>
<dc:title>Large Test Workbook</dc:title>
<dcterms:created xsi:type="dcterms:W3CDTF">2024-01-01T00:00:00Z</dcterms:created>
</cp:coreProperties>"#)?;
    
    // xl/workbook.xml
    zip.start_file("xl/workbook.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="Data" sheetId="1" r:id="rId1"/>
</sheets>
</workbook>"#)?;
    
    // xl/_rels/workbook.xml.rels
    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#)?;
    
    // xl/sharedStrings.xml - Create strings for column headers
    zip.start_file("xl/sharedStrings.xml", options)?;
    let mut shared_strings = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="10" uniqueCount="10">"#);
    
    for i in 0..10 {
        shared_strings.push_str(&format!(r#"<si><t>Column{}</t></si>"#, i));
    }
    shared_strings.push_str("</sst>");
    zip.write_all(shared_strings.as_bytes())?;
    
    // xl/worksheets/sheet1.xml - Create 10,000 cells (100 rows x 100 columns)
    zip.start_file("xl/worksheets/sheet1.xml", options)?;
    let mut worksheet = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#);
    
    // Generate 100 rows with 100 columns each = 10,000 cells
    for row in 1..=100 {
        worksheet.push_str(&format!(r#"<row r="{}">"#, row));
        for col in 0..100 {
            let col_letter = num_to_col(col);
            let cell_ref = format!("{}{}", col_letter, row);
            
            // Mix of numeric and string values
            if col < 10 {
                // First 10 columns are strings (headers in first row, data in others)
                if row == 1 {
                    worksheet.push_str(&format!(r#"<c r="{}" t="s"><v>{}</v></c>"#, cell_ref, col));
                } else {
                    worksheet.push_str(&format!(r#"<c r="{}" t="s"><v>{}</v></c>"#, cell_ref, col % 10));
                }
            } else {
                // Rest are numeric values
                let value = row * 100 + col;
                worksheet.push_str(&format!(r#"<c r="{}"><v>{}</v></c>"#, cell_ref, value));
            }
        }
        worksheet.push_str("</row>");
    }
    
    worksheet.push_str("</sheetData></worksheet>");
    zip.write_all(worksheet.as_bytes())?;
    
    zip.finish()?;
    println!("Created large_sample.xlsx with 10,000 cells");
    Ok(())
}

fn create_large_pptx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating large_sample.pptx with 100 slides...");
    
    let file = File::create("test_data/document/large_sample.pptx")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    let mut content_types = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>"#);
    
    for i in 1..=100 {
        content_types.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
            i
        ));
    }
    content_types.push_str(r#"<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#);
    zip.write_all(content_types.as_bytes())?;
    
    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#)?;
    
    // docProps/core.xml
    zip.start_file("docProps/core.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<dc:creator>Performance Test</dc:creator>
<dc:title>Large Test Presentation</dc:title>
<dcterms:created xsi:type="dcterms:W3CDTF">2024-01-01T00:00:00Z</dcterms:created>
</cp:coreProperties>"#)?;
    
    // ppt/presentation.xml
    zip.start_file("ppt/presentation.xml", options)?;
    let mut presentation = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<p:sldIdLst>"#);
    
    for i in 1..=100 {
        presentation.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            255 + i, i
        ));
    }
    presentation.push_str("</p:sldIdLst></p:presentation>");
    zip.write_all(presentation.as_bytes())?;
    
    // ppt/_rels/presentation.xml.rels
    zip.start_file("ppt/_rels/presentation.xml.rels", options)?;
    let mut pres_rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    
    for i in 1..=100 {
        pres_rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
            i, i
        ));
    }
    pres_rels.push_str("</Relationships>");
    zip.write_all(pres_rels.as_bytes())?;
    
    // Create 100 slides
    for i in 1..=100 {
        let slide_path = format!("ppt/slides/slide{}.xml", i);
        zip.start_file(&slide_path, options)?;
        
        let slide_content = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p><a:r><a:t>Slide {} Title</a:t></a:r></a:p>
<a:p><a:r><a:t>This is the content for slide number {}</a:t></a:r></a:p>
<a:p><a:r><a:t>Additional text to make the slide more realistic</a:t></a:r></a:p>
<a:p><a:r><a:t>Point 1: Important information</a:t></a:r></a:p>
<a:p><a:r><a:t>Point 2: More details here</a:t></a:r></a:p>
<a:p><a:r><a:t>Point 3: Conclusion for this slide</a:t></a:r></a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#, i, i);
        
        zip.write_all(slide_content.as_bytes())?;
    }
    
    zip.finish()?;
    println!("Created large_sample.pptx with 100 slides");
    Ok(())
}

// Helper function to convert column number to Excel column letter
fn num_to_col(mut num: usize) -> String {
    let mut result = String::new();
    while num >= 26 {
        result.insert(0, (b'A' + (num % 26) as u8) as char);
        num = num / 26 - 1;
    }
    result.insert(0, (b'A' + num as u8) as char);
    result
}
