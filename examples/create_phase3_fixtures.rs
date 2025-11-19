use std::fs::File;
use std::io::Write;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Phase 3 test fixtures...");
    
    create_sample_ods()?;
    create_sample_odp()?;
    create_sample_xls()?;
    create_sample_doc()?;
    create_sample_ppt()?;
    
    println!("All Phase 3 test fixtures created successfully!");
    Ok(())
}

fn create_sample_ods() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.ods...");
    
    let file = File::create("test_data/document/sample.ods")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // mimetype (must be first and uncompressed)
    let uncompressed = FileOptions::default()
        .compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", uncompressed)?;
    zip.write_all(b"application/vnd.oasis.opendocument.spreadsheet")?;
    
    // META-INF/manifest.xml
    zip.start_file("META-INF/manifest.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
<manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#)?;
    
    // meta.xml
    zip.start_file("meta.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" 
                      xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/">
<office:meta>
<dc:creator>Test Author</dc:creator>
<dc:title>Sample ODS Spreadsheet</dc:title>
<dc:subject>Test Subject</dc:subject>
<meta:creation-date>2024-01-15T00:00:00</meta:creation-date>
</office:meta>
</office:document-meta>"#)?;
    
    // content.xml with multiple tables
    zip.start_file("content.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                         xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
                         xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body>
<office:spreadsheet>
<table:table table:name="Sheet1">
<table:table-row>
<table:table-cell office:value-type="string"><text:p>Name</text:p></table:table-cell>
<table:table-cell office:value-type="string"><text:p>Age</text:p></table:table-cell>
<table:table-cell office:value-type="string"><text:p>City</text:p></table:table-cell>
</table:table-row>
<table:table-row>
<table:table-cell office:value-type="string"><text:p>Alice</text:p></table:table-cell>
<table:table-cell office:value-type="float" office:value="30"><text:p>30</text:p></table:table-cell>
<table:table-cell office:value-type="string"><text:p>New York</text:p></table:table-cell>
</table:table-row>
<table:table-row>
<table:table-cell office:value-type="string"><text:p>Bob</text:p></table:table-cell>
<table:table-cell office:value-type="float" office:value="25"><text:p>25</text:p></table:table-cell>
<table:table-cell office:value-type="string"><text:p>San Francisco</text:p></table:table-cell>
</table:table-row>
</table:table>
<table:table table:name="Sheet2">
<table:table-row>
<table:table-cell office:value-type="string"><text:p>Product</text:p></table:table-cell>
<table:table-cell office:value-type="string"><text:p>Price</text:p></table:table-cell>
</table:table-row>
<table:table-row>
<table:table-cell office:value-type="string"><text:p>Widget</text:p></table:table-cell>
<table:table-cell office:value-type="float" office:value="19.99"><text:p>19.99</text:p></table:table-cell>
</table:table-row>
<table:table-row>
<table:table-cell office:value-type="string"><text:p>Gadget</text:p></table:table-cell>
<table:table-cell office:value-type="float" office:value="29.99"><text:p>29.99</text:p></table:table-cell>
</table:table-row>
</table:table>
</office:spreadsheet>
</office:body>
</office:document-content>"#)?;
    
    zip.finish()?;
    println!("Created sample.ods");
    Ok(())
}

fn create_sample_odp() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.odp...");
    
    let file = File::create("test_data/document/sample.odp")?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    
    // mimetype (must be first and uncompressed)
    let uncompressed = FileOptions::default()
        .compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", uncompressed)?;
    zip.write_all(b"application/vnd.oasis.opendocument.presentation")?;
    
    // META-INF/manifest.xml
    zip.start_file("META-INF/manifest.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.presentation"/>
<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
<manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#)?;
    
    // meta.xml
    zip.start_file("meta.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" 
                      xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/">
<office:meta>
<dc:creator>Test Presenter</dc:creator>
<dc:title>Test Presentation</dc:title>
<meta:creation-date>2024-01-01T00:00:00</meta:creation-date>
</office:meta>
</office:document-meta>"#)?;
    
    // content.xml with multiple slides
    zip.start_file("content.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                         xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
                         xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body>
<office:presentation>
<draw:page draw:name="Slide1">
<draw:frame>
<draw:text-box>
<text:p>Welcome to Our Presentation</text:p>
<text:p>Introduction to the topic</text:p>
</draw:text-box>
</draw:frame>
</draw:page>
<draw:page draw:name="Slide2">
<draw:frame>
<draw:text-box>
<text:p>Key Points</text:p>
<text:p>Point 1: Important information</text:p>
<text:p>Point 2: More details</text:p>
<text:p>Point 3: Conclusion</text:p>
</draw:text-box>
</draw:frame>
</draw:page>
<draw:page draw:name="Slide3">
<draw:frame>
<draw:text-box>
<text:p>Thank You</text:p>
<text:p>Questions?</text:p>
</draw:text-box>
</draw:frame>
</draw:page>
</office:presentation>
</office:body>
</office:document-content>"#)?;
    
    zip.finish()?;
    println!("Created sample.odp");
    Ok(())
}

fn create_sample_xls() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.xls...");
    
    // XLS files use OLE2 format (Compound File Binary Format)
    // We'll create a minimal OLE2 structure with XLS signature
    let mut file = File::create("test_data/document/sample.xls")?;
    
    // OLE2 header
    let ole2_header: [u8; 80] = [
        // Signature
        0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1,
        // CLSID (16 bytes of zeros)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Minor version (0x003E)
        0x3E, 0x00,
        // Major version (0x0003 for version 3)
        0x03, 0x00,
        // Byte order (0xFFFE for little-endian)
        0xFE, 0xFF,
        // Sector size (0x0009 = 512 bytes)
        0x09, 0x00,
        // Mini sector size (0x0006 = 64 bytes)
        0x06, 0x00,
        // Reserved (6 bytes)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Total sectors (0 for version 3)
        0x00, 0x00, 0x00, 0x00,
        // FAT sectors (0 for version 3)
        0x00, 0x00, 0x00, 0x00,
        // First directory sector
        0x00, 0x00, 0x00, 0x00,
        // Transaction signature
        0x00, 0x00, 0x00, 0x00,
        // Mini stream cutoff size (4096 bytes)
        0x00, 0x10, 0x00, 0x00,
        // First mini FAT sector
        0xFE, 0xFF, 0xFF, 0xFF,
        // Number of mini FAT sectors
        0x00, 0x00, 0x00, 0x00,
        // First DIFAT sector
        0xFE, 0xFF, 0xFF, 0xFF,
        // Number of DIFAT sectors
        0x00, 0x00, 0x00, 0x00,
        // DIFAT array start
        0x00, 0x00, 0x00, 0x00,
    ];
    
    // Write header and pad to 512 bytes
    file.write_all(&ole2_header)?;
    // Fill rest with 0xFF
    for _ in 0..(512 - 80) {
        file.write_all(&[0xFF])?;
    }
    
    println!("Created sample.xls (minimal OLE2 structure)");
    Ok(())
}

fn create_sample_doc() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.doc...");
    
    // DOC files use OLE2 format with Word-specific streams
    let mut file = File::create("test_data/document/sample.doc")?;
    
    // OLE2 header (same structure as XLS)
    let ole2_header: [u8; 80] = [
        // Signature
        0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1,
        // CLSID (16 bytes of zeros)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Minor version
        0x3E, 0x00,
        // Major version
        0x03, 0x00,
        // Byte order
        0xFE, 0xFF,
        // Sector size
        0x09, 0x00,
        // Mini sector size
        0x06, 0x00,
        // Reserved
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Total sectors
        0x00, 0x00, 0x00, 0x00,
        // FAT sectors
        0x00, 0x00, 0x00, 0x00,
        // First directory sector
        0x00, 0x00, 0x00, 0x00,
        // Transaction signature
        0x00, 0x00, 0x00, 0x00,
        // Mini stream cutoff size
        0x00, 0x10, 0x00, 0x00,
        // First mini FAT sector
        0xFE, 0xFF, 0xFF, 0xFF,
        // Number of mini FAT sectors
        0x00, 0x00, 0x00, 0x00,
        // First DIFAT sector
        0xFE, 0xFF, 0xFF, 0xFF,
        // Number of DIFAT sectors
        0x00, 0x00, 0x00, 0x00,
        // DIFAT array start
        0x00, 0x00, 0x00, 0x00,
    ];
    
    file.write_all(&ole2_header)?;
    // Fill rest with 0xFF to complete 512-byte header
    for _ in 0..(512 - 80) {
        file.write_all(&[0xFF])?;
    }
    
    println!("Created sample.doc (minimal OLE2 structure)");
    Ok(())
}

fn create_sample_ppt() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample.ppt...");
    
    // PPT files use OLE2 format with PowerPoint-specific streams
    let mut file = File::create("test_data/document/sample.ppt")?;
    
    // OLE2 header (same structure)
    let ole2_header: [u8; 80] = [
        // Signature
        0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1,
        // CLSID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Minor version
        0x3E, 0x00,
        // Major version
        0x03, 0x00,
        // Byte order
        0xFE, 0xFF,
        // Sector size
        0x09, 0x00,
        // Mini sector size
        0x06, 0x00,
        // Reserved
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Total sectors
        0x00, 0x00, 0x00, 0x00,
        // FAT sectors
        0x00, 0x00, 0x00, 0x00,
        // First directory sector
        0x00, 0x00, 0x00, 0x00,
        // Transaction signature
        0x00, 0x00, 0x00, 0x00,
        // Mini stream cutoff size
        0x00, 0x10, 0x00, 0x00,
        // First mini FAT sector
        0xFE, 0xFF, 0xFF, 0xFF,
        // Number of mini FAT sectors
        0x00, 0x00, 0x00, 0x00,
        // First DIFAT sector
        0xFE, 0xFF, 0xFF, 0xFF,
        // Number of DIFAT sectors
        0x00, 0x00, 0x00, 0x00,
        // DIFAT array start
        0x00, 0x00, 0x00, 0x00,
    ];
    
    file.write_all(&ole2_header)?;
    // Fill rest with 0xFF to complete 512-byte header
    for _ in 0..(512 - 80) {
        file.write_all(&[0xFF])?;
    }
    
    println!("Created sample.ppt (minimal OLE2 structure)");
    Ok(())
}
