use anyhow::{anyhow, Context, Result};
use std::{fs, path::Path, process::Command};
// use tempfile::NamedTempFile;
use tracing::{info, warn};

use crate::scanner::{scan_text, ScanResult};

fn run_command(program: &str, args: &[String]) -> Result<()> {
    let output = Command::new(program).args(args).output()
        .with_context(|| format!("failed to execute `{program}`; install it and ensure it is on PATH"))?;
    if !output.status.success() {
        return Err(anyhow!("{program} failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(())
}

fn tesseract_image(image_path: &Path, lang: &str) -> Result<String> {
    let output = Command::new("tesseract")
        .arg(image_path).arg("stdout")
        .arg("-l").arg(lang)
        .args(["--oem", "3", "--psm", "6"])
        .output().context("failed to execute tesseract; install Tesseract OCR")?;
    if !output.status.success() {
        return Err(anyhow!("tesseract failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_image(path: &Path) -> bool {
    matches!(path.extension().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp")
}

fn render_pdf_page(pdf: &Path, page_number: usize, dpi: u32, prefix: &Path) -> Result<std::path::PathBuf> {
    // pdftoppm uses 1-based page numbering. Output path gets a page suffix.
    let prefix_str = prefix.to_string_lossy().to_string();
    let args = vec![
        "-f".into(), page_number.to_string(),
        "-l".into(), page_number.to_string(),
        "-r".into(), dpi.to_string(),
        "-singlefile".into(),
        "-png".into(),
        pdf.to_string_lossy().to_string(),
        prefix_str.clone(),
    ];
    run_command("pdftoppm", &args)?;
    Ok(std::path::PathBuf::from(format!("{prefix_str}.png")))
}

pub fn extract_text_and_scan(bytes: &[u8], filename: &str, dpi: u32, max_ocr_pages: usize) -> Result<ScanResult> {
    let suffix = Path::new(filename).extension().and_then(|s| s.to_str())
        .map(|s| format!(".{s}")).unwrap_or_else(|| ".pdf".into());
    let mut tmp = tempfile::Builder::new().suffix(&suffix).tempfile()?;
    fs::write(tmp.path(), bytes)?;
    let path = tmp.path();
    let lang = std::env::var("TESSERACT_LANG").unwrap_or_else(|_| "eng".into());

    let text = if is_image(Path::new(filename)) {
        tesseract_image(path, &lang)?
    } else {
        extract_pdf_text(path, dpi, max_ocr_pages, &lang)?
    };
    Ok(scan_text(&text))
}

fn extract_pdf_text(path: &Path, dpi: u32, max_ocr_pages: usize, lang: &str) -> Result<String> {
    // Count pages with Poppler, then extract native text per page. This preserves
    // the Python implementation's behavior for mixed digital/scanned PDFs.
    let page_count = Command::new("pdfinfo").arg(path).output()
        .context("failed to execute pdfinfo; install Poppler utilities")?;
    if !page_count.status.success() {
        return Err(anyhow!("pdfinfo failed: {}", String::from_utf8_lossy(&page_count.stderr)));
    }
    let info_text = String::from_utf8_lossy(&page_count.stdout);
    let total_pages: usize = info_text.lines()
        .find_map(|line| line.strip_prefix("Pages:").and_then(|v| v.trim().parse().ok()))
        .ok_or_else(|| anyhow!("could not determine PDF page count"))?;

    let mut pages = vec![String::new(); total_pages];
    let mut needs_ocr = Vec::new();

    for page in 1..=total_pages {
        let page_string = page.to_string();
        let output = Command::new("pdftotext")
            .args(["-f", page_string.as_str(), "-l", page_string.as_str(), "-layout"])
            .arg(path).arg("-")
            .output().context("failed to execute pdftotext; install Poppler utilities")?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                pages[page - 1] = text;
                continue;
            }
        }
        needs_ocr.push(page);
    }

    let selected = select_pages_from_candidates(&needs_ocr, max_ocr_pages);
    info!("OCR needed for {}/{} pages; processing {} at {} DPI",
        needs_ocr.len(), total_pages, selected.len(), dpi);

    let temp_dir = tempfile::tempdir()?;
    for page in selected {
        let prefix = temp_dir.path().join(format!("page-{page}"));
        let image = render_pdf_page(path, page, dpi, &prefix)?;
        pages[page - 1] = tesseract_image(&image, lang)?;
    }

    Ok(pages.into_iter().enumerate()
        .map(|(i, text)| if text.is_empty() { String::new() } else { format!("[Page {}]\n{}", i + 1, text) })
        .collect::<Vec<_>>().join("\n"))
}

fn select_pages_from_candidates(candidates: &[usize], max: usize) -> Vec<usize> {
    if candidates.len() <= max { return candidates.to_vec(); }
    if max == 0 { return Vec::new(); }
    if max <= 6 { return candidates.iter().take(max).copied().collect(); }
    let mut selected = Vec::new();
    selected.extend(candidates.iter().take(3).copied());
    selected.extend(candidates.iter().rev().take(3).copied());
    for page in candidates.iter().skip(3).take(candidates.len().saturating_sub(6)) {
        if selected.len() >= max { break; }
        selected.push(*page);
    }
    selected.sort_unstable();
    selected.dedup();
    selected.truncate(max);
    selected
}

pub fn extract_and_scan(bytes: &[u8], filename: &str, dpi: u32, max_pages: usize) -> Result<ScanResult> {
    extract_text_and_scan(bytes, filename, dpi, max_pages)
}
