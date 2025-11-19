use std::fs::File;
use std::io::Write;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Phase 2 test fixtures...");
    
    create_sample_xlsx()?;
    create_sample_pptx()?;
    create_encrypted_xlsx()?;
    create_corrupted_xlsx()?;
    
    println!("All test fixtures created successfully!");
    Ok(())
}

fn create_sample_xlsx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.xlsx...");
    
    let file = File::create("test_data/document/sample.xlsx")?;
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
<Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
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
<dc:creator>Test Author</dc:creator>
<dc:title>Test Workbook</dc:title>
<dc:subject>Test Subject</dc:subject>
<dcterms:created xsi:type="dcterms:W3CDTF">2024-01-01T00:00:00Z</dcterms:created>
</cp:coreProperties>"#)?;
    
    // xl/workbook.xml
    zip.start_file("xl/workbook.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="Sales Data" sheetId="1" r:id="rId1"/>
<sheet name="Summary" sheetId="2" r:id="rId2"/>
</sheets>
</workbook>"#)?;
    
    // xl/_rels/workbook.xml.rels
    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#)?;
    
    // xl/sharedStrings.xml
    zip.start_file("xl/sharedStrings.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="6" uniqueCount="6">
<si><t>Product</t></si>
<si><t>Quantity</t></si>
<si><t>Price</t></si>
<si><t>Laptop</t></si>
<si><t>Mouse</t></si>
<si><t>Total Revenue</t></si>
</sst>"#)?;
    
    // xl/worksheets/sheet1.xml - Sales Data with formulas
    zip.start_file("xl/worksheets/sheet1.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>
<row r="1">
<c r="A1" t="s"><v>0</v></c>
<c r="B1" t="s"><v>1</v></c>
<c r="C1" t="s"><v>2</v></c>
</row>
<row r="2">
<c r="A2" t="s"><v>3</v></c>
<c r="B2"><v>5</v></c>
<c r="C2"><v>999.99</v></c>
</row>
<row r="3">
<c r="A3" t="s"><v>4</v></c>
<c r="B3"><v>10</v></c>
<c r="C3"><v>25.50</v></c>
</row>
</sheetData>
</worksheet>"#)?;
    
    // xl/worksheets/sheet2.xml - Summary with formula
    zip.start_file("xl/worksheets/sheet2.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>
<row r="1">
<c r="A1" t="s"><v>5</v></c>
<c r="B1"><f>SUM(B2:B3)</f><v>5254.95</v></c>
</row>
</sheetData>
</worksheet>"#)?;
    
    zip.finish()?;
    println!("Created sample.xlsx");
    Ok(())
}

fn create_sample_pptx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.pptx...");
    
    let file = File::create("test_data/document/sample.pptx")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/notesSlides/notesSlide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#)?;
    
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
<dc:creator>Test Presenter</dc:creator>
<dc:title>Test Presentation</dc:title>
<dc:subject>Test Topic</dc:subject>
<dcterms:created xsi:type="dcterms:W3CDTF">2024-01-01T00:00:00Z</dcterms:created>
</cp:coreProperties>"#)?;
    
    // ppt/presentation.xml
    zip.start_file("ppt/presentation.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<p:sldIdLst>
<p:sldId id="256" r:id="rId1"/>
<p:sldId id="257" r:id="rId2"/>
</p:sldIdLst>
</p:presentation>"#)?;
    
    // ppt/_rels/presentation.xml.rels
    zip.start_file("ppt/_rels/presentation.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
</Relationships>"#)?;
    
    // ppt/slides/slide1.xml
    zip.start_file("ppt/slides/slide1.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p><a:r><a:t>Welcome to Our Presentation</a:t></a:r></a:p>
<a:p><a:r><a:t>Introduction to the topic</a:t></a:r></a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#)?;
    
    // ppt/slides/_rels/slide1.xml.rels
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/>
</Relationships>"#)?;
    
    // ppt/notesSlides/notesSlide1.xml
    zip.start_file("ppt/notesSlides/notesSlide1.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p><a:r><a:t>Speaker notes for slide 1: Remember to introduce yourself and explain the agenda.</a:t></a:r></a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:notes>"#)?;
    
    // ppt/slides/slide2.xml
    zip.start_file("ppt/slides/slide2.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p><a:r><a:t>Key Points</a:t></a:r></a:p>
<a:p><a:r><a:t>Point 1: Important information</a:t></a:r></a:p>
<a:p><a:r><a:t>Point 2: More details</a:t></a:r></a:p>
<a:p><a:r><a:t>Point 3: Conclusion</a:t></a:r></a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#)?;
    
    zip.finish()?;
    println!("Created sample.pptx");
    Ok(())
}

fn create_encrypted_xlsx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating encrypted.xlsx...");
    
    // Create a minimal encrypted XLSX file
    // In reality, encrypted Office files use OLE2 encryption
    // For testing purposes, we'll create a file that starts with the OLE2 header
    // which indicates encryption
    let file = File::create("test_data/document/encrypted.xlsx")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // Create a minimal structure that will fail to parse properly
    // simulating an encrypted file
    zip.start_file("EncryptionInfo", options)?;
    zip.write_all(b"This file is encrypted")?;
    
    zip.start_file("EncryptedPackage", options)?;
    zip.write_all(b"Encrypted data here")?;
    
    zip.finish()?;
    println!("Created encrypted.xlsx");
    Ok(())
}

fn create_corrupted_xlsx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating corrupted.xlsx...");
    
    let file = File::create("test_data/document/corrupted.xlsx")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // Create a file with invalid XML that will fail parsing
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(b"<?xml version=\"1.0\"?><Types><CORRUPTED")?;
    
    zip.start_file("xl/workbook.xml", options)?;
    zip.write_all(b"<workbook><INVALID XML HERE")?;
    
    zip.finish()?;
    println!("Created corrupted.xlsx");
    Ok(())
}
