# SensiShieldProxy

A Lightweight AI upload interception proxy for detecting sensitive legal documents and high-risk data before they are uploaded to platforms for ChatGPT, Claude, and Perplexity.
The system uses mitmproxy for interception, FastAPI for scanning, OCR for scanned PDFs, Redis caching for repeated uploads, and heuristic/legal detection for agreements, NDAs, and confidential documents.

---

<img width="892" height="260" alt="image" src="https://github.com/user-attachments/assets/f9fff0e2-9c0b-4915-998d-cceed5af6059" />

A very legal document being uploaded.



<img width="999" height="184" alt="image" src="https://github.com/user-attachments/assets/f47d0268-8cb3-4b0e-9c5d-ef5f979faa57" />

Proxy blocks the upload of the very legal upload.


# How It Works

```text id="e3f1"
User Uploads a File
↓
mitmproxy intercepts request
↓
PDF extracted from upload body
↓
FastAPI scanner receives file
↓
OCR/Text extraction + legal classification
↓
Decision is returned - allow or block
↓
Upload forwarded or blocked
```

---

# Project Structure

## `dlpProxy.py`

Main interception script using mitmproxy

Handles:

* OpenAI, Claude and Perplexity uploads & multipart uploads
* Text prompt interception
* Upload blocking logic
* Request forwarding to FastAPI scanner

---

## `server.py`

FastAPI scanning backend.

Handles:

* File upload scanning
* OCR pipeline execution
* Redis caching
* Legal document scoring
* Presidio PII analysis
* Final allow/block decision

---

## `extractTextFromPDF.py`

OCR and document analysis engine.

Handles:

* Native PDF text extraction
* OCR fallback using Tesseract
* Legal document heuristic scoring
* Page classification
* Hybrid extraction pipeline

---

## `redisCache.py`

Redis helper module.

Handles:

* Cache storage - for any blocked files + files above sizes of
* Cached scan retrieval
* File hash lookups

---

## `dlpProxy.log`

Central runtime log file.

Contains:

* Upload events
* OCR timings
* Detection decisions
* Cache hits/misses
* Errors/debugging info

---

# Requirements

## System Dependencies

Install:

* Python 3.11+
* Redis
* Tesseract OCR
* Poppler

### Fedora

```bash id="j4r2"
sudo dnf install redis tesseract poppler-utils
```

### Windows (Use Chocolatey)

```bash id="hello"
choco install redis-64 tesseract poppler -y
```

---

# Python Dependencies

Install requirements:

```bash id="h8v1"
pip install -r requirements.txt
```

---

# Setup

## 1. Start Redis

```bash id="m5t7"
redis-server
```

---

## 2. Start FastAPI Scanner

```bash id="w1f8"
uvicorn server:app --host 127.0.0.1 --port 8000
```

---

## 3. Start mitmproxy

```bash id="n7q3"
mitmweb -s dlpProxy.py --listen-host 0.0.0.0 --listen-port 8080
```

---

## 4. Configure Device Proxy

Set device/browser proxy to:

```text id="p9z4"
IP: <SERVER_IP>
PORT: 8080
```

---

## 5. Install mitmproxy Certificate

Open:

```text id="u6d1"
http://mitm.it
```

Install the certificate on the client device/browser.

---

# Features

* OpenAI upload interception
* Claude multipart upload parsing
* Perplexity upload parsing
* OCR support for scanned PDFs
* Legal agreement/NDA detection
* Credit card and financial entity detection
* Redis caching for repeated uploads
* Large-file optimized scanning mode
* Upload blocking before cloud submission

---

# Current Limitations

* Perplexity file uploads may be bypassed as they're not tested(I don't have Perplexity Pro)
* VPNs can bypass proxy enforcement
* Browser QUIC/HTTP3 may require disabling
* Heuristic legal detection can produce false positives
* Large scanned PDFs may increase OCR latency
* Windows endpoint enforcement is not implemented yet

# What's left to do
* Fix the decision making from simple heuristic detection
* Include stamp detection
* Include random sampling for larger documents & max size blocking
* Structured logging - current logs suck
* Minor polishing of code
  

# Optional
* Implement HTTPS between components, Redis safety, memory abuse
* API auth

---
Feedbacks are appreciated!

