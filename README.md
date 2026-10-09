# DLP Proxy Rust backend

![awful meme](media/security.png)

This crate ports the **backend** of the supplied Python project to Rust while leaving `dlpProxy.py` as the mitmproxy addon. The Rust service keeps the same endpoint paths expected by that addon:

- `POST /scan` with `{"text":"..."}`
- `POST /scan-file/openai` with multipart field `file`
- `POST /scan-file/claude` with multipart field `file`
- `GET /health`

## What is migrated

- Axum HTTP server and upload endpoints (replaces `server.py` / FastAPI)
- PDF embedded-text extraction (`pdf-extract`)
- OCR fallback via Tesseract CLI + Poppler (`pdftoppm`, `pdfinfo`)
- Legal-term scoring from `extractTextFromPDF.py`
- Redis SHA-256 result cache (7-day TTL)
- Basic Rust PII checks for credit cards (Luhn), IBAN (MOD-97), and common crypto-address patterns

## Prerequisites

Fedora:
```bash
sudo dnf install tesseract poppler-utils redis
```

Debian/Ubuntu:
```bash
sudo apt install tesseract-ocr poppler-utils redis-server
```

Install any Tesseract language packs you need with your distribution package manager. Set `TESSERACT_LANG=eng` (or e.g. `eng+spa+fra`) if required.

## Run

```bash
cargo run --release
```
then
```bash
mitmdump -s dlpProxy.py --listen-host 0.0.0.0 --listen-port 8080
```

Defaults to `127.0.0.1:8000`, matching the Python addon. Optional environment variables:

- `BIND_ADDR=127.0.0.1:8000`
- `REDIS_URL=redis://127.0.0.1:6379/`
- `MAX_UPLOAD_BYTES=157286400`
- `TESSERACT_LANG=eng`
- `RUST_LOG=info`

The mitmproxy addon can continue using the existing local URLs without modification. Keep `dlpProxy.py` in the mitmproxy configuration and start this Rust backend separately.

## OCR strategy and current limitation

The backend uses `pdftotext` per page, so mixed PDFs can retain embedded text on digital pages and OCR only scanned/empty-text pages. OCR fallback renders only a bounded set of those pages, using first/last pages and then middle candidates. Large files use 100 DPI and a 20-page cap; smaller files use 120 DPI and a 12-page cap, matching the Python server's intended settings.

This implementation invokes native tools rather than implementing OCR in Rust; that is deliberate. Rust orchestrates the pipeline while Tesseract and Poppler do the specialized OCR/rendering work.

## Notes

- **`stampDetection.py` integration is not ported yet.**
- **Rust port currently only works for pdf files**
- Uploads are size-limited and hashed before cache lookup.
- Errors return `{"decision":"error","error":"..."}` with a non-2xx response, unlike the Python backend's JSON error response.

## ToDo
- Add compatibility for .docx
- Optimize block classifier
- Stamp detection
- Toggle options for legal/personal data/other options

