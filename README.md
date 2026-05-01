# LinkDrop

A minimalist, high-speed paste service built in Rust. Designed for privacy, OLED-dark aesthetics, and a "stealth" workflow.

## Run with Docker

```bash
docker run -p 8080:8080 \
  -e MICROBIN_PUBLIC_PATH=your-domain.com \
  -v ${PWD}/data:/app/microbin_data \
  linkdrop
```

## Config
- `MICROBIN_PUBLIC_PATH`: Your domain (e.g. `link.example.com`). Used for QR codes and share links.
- `MICROBIN_DATA_DIR`: Path to the SQLite database and storage.
- `MICROBIN_PORT`: Port to listen on (default: 8080).
- `MICROBIN_BIND`: IP to bind to (default: 0.0.0.0).

## Features
- **Minimalist UI**: Pure black background, zero distraction.
- **Ephemeral**: Burn-after-read and auto-expiry support.
- **Mobile Friendly**: Integrated QR codes for instant transfer.
- **Read-Only Mode**: Lock pastes so they can't be edited.
- **Security**: 1MB payload limits and slug sanitization.
- **Pure Share**: Special preview mode (`?created=true`) that doesn't count as a "view".

## Development
Requires Rust and Cargo.

```bash
cargo build --release
./target/release/linkdrop
```

---
[BSD 3-Clause License](LICENSE)
