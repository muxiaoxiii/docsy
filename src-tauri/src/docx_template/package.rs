use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::Path;

use zip::write::FileOptions;
use zip::ZipArchive;

use super::TemplateManifest;
use super::MANIFEST_PATH;
use super::MAX_BINARY_ENTRY_BYTES;
use super::MAX_DOCSYTPL_BYTES;
use super::MAX_DOCX_BYTES;
use super::MAX_MANIFEST_BYTES;
use super::MAX_PACKAGE_UNCOMPRESSED_BYTES;
use super::MAX_PACKAGE_XML_BYTES;
use super::MAX_XML_ENTRY_BYTES;
use super::MAX_ZIP_ENTRIES;

pub type Package = HashMap<String, Vec<u8>>;

pub fn read_docx_package(path: &Path) -> Result<Package> {
    let bytes = super::read_file_with_limit(path, MAX_DOCX_BYTES, "Word 文件")?;
    read_zip_package(&bytes, false, "Word 文件")
}

#[allow(dead_code)] // byte-backed entry point is kept for callers that already own an upload buffer
pub fn read_docx_package_from_bytes(bytes: &[u8]) -> Result<Package> {
    read_zip_package(bytes, false, "Word 文件")
}

pub fn write_docx_package(path: &Path, pkg: &Package) -> Result<()> {
    write_zip_package(path, pkg)
}

pub fn read_docsytpl_package(path: &Path) -> Result<(TemplateManifest, Package)> {
    let bytes = super::read_file_with_limit(path, MAX_DOCSYTPL_BYTES, "Docsy 模板文件")?;
    let pkg = read_zip_package(&bytes, false, "Docsy 模板文件")?;

    let manifest_json = pkg
        .get(MANIFEST_PATH)
        .context("模板文件缺少 manifest.json")?;
    let manifest: TemplateManifest =
        serde_json::from_slice(manifest_json).context("manifest.json 格式不正确")?;
    if manifest.format_version > 2 {
        anyhow::bail!(
            "模板版本不兼容：当前 Docsy 支持版本 1-2，但该模板是版本 {}",
            manifest.format_version
        );
    }
    Ok((manifest, pkg))
}

pub fn write_docsytpl_package(
    path: &Path,
    manifest: &TemplateManifest,
    pkg: &Package,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp_path = atomic_temp_path(path);
    let file = std::fs::File::create(&temp_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    writer.start_file(MANIFEST_PATH, options)?;
    writer.write_all(serde_json::to_vec_pretty(manifest)?.as_slice())?;

    for (name, data) in pkg {
        if name == MANIFEST_PATH {
            continue;
        }
        writer.start_file(name, options)?;
        writer.write_all(data)?;
    }
    writer.finish()?;
    replace_atomically(&temp_path, path)?;
    Ok(())
}

fn read_zip_package(bytes: &[u8], filter_xml_only: bool, label: &str) -> Result<Package> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).with_context(|| format!("读取{}失败", label))?;
    ensure_entry_count(archive.len(), label)?;

    let mut pkg = HashMap::new();
    let mut total_size = 0_u64;
    let mut total_xml_size = 0_u64;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let name = file.name().to_string();
        if file.is_dir() {
            continue;
        }
        let size = file.size();
        let limit = if super::is_word_xml_part(&name) || name == MANIFEST_PATH {
            if name == MANIFEST_PATH {
                MAX_MANIFEST_BYTES
            } else {
                MAX_XML_ENTRY_BYTES
            }
        } else {
            if filter_xml_only {
                continue;
            }
            MAX_BINARY_ENTRY_BYTES
        };
        total_size = total_size.saturating_add(size);
        if total_size > MAX_PACKAGE_UNCOMPRESSED_BYTES {
            anyhow::bail!("{}解压后的总大小超过限制", label);
        }
        if super::is_word_xml_part(&name) || name == MANIFEST_PATH {
            total_xml_size = total_xml_size.saturating_add(size);
            if total_xml_size > MAX_PACKAGE_XML_BYTES {
                anyhow::bail!("{}中的 XML 总大小超过限制", label);
            }
        }

        let data = super::read_vec_with_limit(
            &mut file,
            size,
            limit,
            &format!("{} 条目: {}", label, name),
        )?;
        pkg.insert(name, data);
    }
    Ok(pkg)
}

fn write_zip_package(path: &Path, pkg: &Package) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp_path = atomic_temp_path(path);
    let file = std::fs::File::create(&temp_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in pkg {
        writer.start_file(name, options)?;
        writer.write_all(data)?;
    }
    writer.finish()?;
    replace_atomically(&temp_path, path)?;
    Ok(())
}

fn atomic_temp_path(path: &Path) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("package");
    path.with_file_name(format!(".{file_name}.{nonce}.tmp"))
}

fn replace_atomically(temp_path: &Path, path: &Path) -> Result<()> {
    if let Err(error) = std::fs::rename(temp_path, path) {
        let _ = std::fs::remove_file(temp_path);
        return Err(error.into());
    }
    Ok(())
}

fn ensure_entry_count(count: usize, label: &str) -> Result<()> {
    if count > MAX_ZIP_ENTRIES {
        anyhow::bail!(
            "{}包含过多内部文件({} 个)，已拒绝处理（限制 {} 个）",
            label,
            count,
            MAX_ZIP_ENTRIES
        );
    }
    Ok(())
}
