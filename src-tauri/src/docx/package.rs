/// Read and write OPC (docx) zip packages.
use anyhow::Result;

pub fn read_entry(archive: &mut zip::ZipArchive<impl std::io::Read + std::io::Seek>, name: &str) -> Result<String> {
    let mut file = archive.by_name(name)?;
    let mut content = String::new();
    std::io::Read::read_to_string(&mut file, &mut content)?;
    Ok(content)
}

pub fn read_entry_bytes(archive: &mut zip::ZipArchive<impl std::io::Read + std::io::Seek>, name: &str) -> Result<Vec<u8>> {
    let mut file = archive.by_name(name)?;
    let mut content = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut content)?;
    Ok(content)
}
