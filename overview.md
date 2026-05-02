# LinkDrop: Comprehensive Project Overview

This document provides a detailed, technical, and structural breakdown of the **LinkDrop** project. It is designed to serve as the foundational material for generating comprehensive project documentation, academic reports, or extensive technical case studies.

---

## 1. Executive Summary
**LinkDrop** is a high-performance, minimalist, self-hosted web application built in Rust. It functions as a secure "digital clipboard" or pastebin service, allowing users to quickly share text, code snippets, or URLs. 

The project distinguishes itself through a "Stealth-Premium" design philosophy:
*   **Performance:** Powered by the Actix-web framework, ensuring microsecond response times and high concurrency.
*   **Minimalism:** A distraction-free, pure dark-mode UI that prioritizes content visibility over complex navigation.
*   **Security:** Built-in payload limits, sanitized URLs, and ephemeral sharing (Burn-After-Read).

## 2. Technology Stack
The application is built on a modern, robust technology stack:

### Backend (Systems Programming)
*   **Rust:** The primary language, chosen for its memory safety, zero-cost abstractions, and fearless concurrency.
*   **Actix-Web:** A powerful, pragmatic, and extremely fast web framework for Rust. It handles routing, middleware, and HTTP requests.
*   **Askama:** A type-safe, compiled template engine for Rust. It renders HTML views efficiently at runtime.
*   **Rusqlite / SQLite:** A lightweight, serverless database engine used for persistent storage of pastes.

### Frontend (UI/UX)
*   **HTML5:** Semantic structuring of the application views.
*   **Tailwind CSS (via CDN):** A utility-first CSS framework used for rapid UI styling, enforcing a strict dark theme (`class="dark"`).
*   **Material Symbols:** Google's icon library for minimal, recognizable interface iconography.

### Infrastructure
*   **Docker:** Containerization for consistent, isolated, and easy deployment across any environment.

---

## 3. Detailed File Structure & Architecture

The repository is organized following standard Rust project conventions, separating routing logic, data utilities, and frontend templates.

```text
LinkDrop/
├── Cargo.toml                  # Rust dependency and build configuration manager
├── Dockerfile                  # Instructions for building the containerized application
├── README.md                   # Quick-start guide and basic project introduction
├── AGENTS.md                   # Internal rules and CLI flag documentation
├── data/                       # Directory mapped to Docker volumes for SQLite persistence
├── templates/                  # Askama HTML templates (Frontend)
│   ├── create.html             # The primary interface for pasting and configuring drops
│   ├── view.html               # The interface for reading a paste
│   ├── share.html              # The post-creation screen displaying the URL and QR code
│   ├── expired.html            # The fallback view when a paste has expired or been burned
│   └── home.html               # The landing page redirecting users to the creation flow
└── src/                        # Rust Source Code (Backend)
    ├── main.rs                 # Application entry point, server initialization, and global state setup
    ├── args.rs                 # CLI argument parsing and environment variable configuration
    ├── endpoints/              # HTTP Route Handlers
    │   ├── mod.rs              # Module definition
    │   └── core_routes.rs      # Contains the main logic for GET/POST requests on slugs
    └── util/                   # Shared utilities and data access layers
        ├── mod.rs              # Module definition
        ├── db.rs               # Database abstraction trait/interface
        ├── db_sqlite.rs        # SQLite-specific implementation of the database interface
        ├── misc.rs             # Helper functions (e.g., QR code generation)
        └── models.rs           # Data structures representing Pastes, Configurations, etc.
```

### Key Component Roles:
*   **`main.rs`**: Initializes the global state (`Mutex<HashMap>`), configures Actix-web payload limits (e.g., 1MB max), and starts the HTTP server on the configured port.
*   **`core_routes.rs`**: The brain of the application. It handles the dynamic `/{slug}` route. It determines if a user is trying to create a new paste, view an existing one, edit one, or view the share screen based on the HTTP method and query parameters.
*   **`db_sqlite.rs`**: Manages the local SQLite `.db` file. It ensures data survives server restarts, mapping Rust models to SQL tables.

---

## 4. Core Workflows and Application Logic

### A. The Creation Flow (POST)
1.  A user navigates to a URL (e.g., `localhost:8080/my-secret`).
2.  If `my-secret` doesn't exist, the server renders `create.html`.
3.  The user inputs text, selects options (Max Views, Expiry), and submits the form.
4.  A POST request is sent to `core_routes.rs`.
5.  The payload is validated (length < 1MB).
6.  A new `Pasta` object is created in memory and written to the SQLite database.
7.  The user is redirected to `/{slug}?created=true`.

### B. The Share Flow (GET `?created=true`)
1.  This is a "Pure UI" state. It explicitly bypasses any logic that would alter the database (like decrementing view counts).
2.  It generates a QR Code SVG string dynamically based on the configured `LINKDROP_PUBLIC_PATH`.
3.  It renders `share.html`, providing the user with their link, QR code, and confirmation of settings.

### C. The Viewing Flow (GET)
1.  A recipient visits `/{slug}`.
2.  The server fetches the paste from the database.
3.  **Validation Check:** Has it expired? Have the maximum views been reached?
    *   If yes: Render `expired.html` and delete the record from the database.
    *   If no: Update "last read time", decrement "views remaining" (if applicable), and render `view.html`.
4.  `view.html` displays the text in a read-only container with a quick "Copy Content" action.

---

## 5. Security & Data Management

LinkDrop implements several layers of security to ensure stability and privacy:

1.  **Strict Payload Limits:** To prevent Denial of Service (DoS) attacks via memory exhaustion, Actix-web is configured to hard-reject any payload exceeding 1MB.
2.  **Slug Sanitization:** Slugs (the URL path) are heavily normalized. They must match the regex `^[a-z0-9-_]{3,50}$`. Special characters are stripped, and spaces are converted to hyphens.
3.  **Reserved Keyword Protection:** Paths like `/api`, `/static`, or `/admin` are blacklisted from being used as custom slugs to prevent routing conflicts.
4.  **Ephemeral Data Mechanics:** 
    *   **Burn-After-Read:** If a paste is configured with a view limit (e.g., 1 view), the database record is permanently deleted the moment it is accessed by a recipient.
    *   **Time-Based Expiry:** Pastes can be set to self-destruct after 1 Hour, 1 Day, etc. A background thread routinely sweeps the database to prune expired data.

---

## 6. Configuration & Environment

The application is highly configurable via environment variables, making it ideal for containerized deployments (Docker/Kubernetes).

*   `LINKDROP_PUBLIC_PATH`: Crucial for deployment behind a reverse proxy (like Nginx) or a tunnel (like Ngrok). It ensures QR codes encode the correct public-facing domain (e.g., `https://my-domain.com`) rather than `localhost`.
*   `LINKDROP_PORT` / `LINKDROP_BIND`: Controls the network interface.
*   `LINKDROP_DATA_DIR`: Defines where the SQLite database is stored, essential for mapping persistent Docker volumes.

---

## 7. Report Expansion Ideas (For 70-Page Requirement)
To expand this into a comprehensive 70-page document, consider structuring chapters around the following deep-dives:
*   **Chapter 1: The Evolution of Digital Clipboards.** (Compare LinkDrop to Pastebin, Ghostbin; discuss the shift towards self-hosting and privacy).
*   **Chapter 2: UI/UX Case Study.** (Analyze the "Stealth" design system. Discuss color theory (OLED black vs. Cyan), cognitive load reduction, and the psychology of minimalist interfaces).
*   **Chapter 3: Rust for Web Services.** (A technical deep dive into why Rust and Actix-web were chosen over Node.js or Python. Include performance benchmarks, memory safety guarantees, and the borrow checker paradigm).
*   **Chapter 4: State Management in Highly Concurrent Systems.** (Analyze how `Mutex` and `HashMap` operate under load, and the trade-offs between in-memory caching and SQLite disk writes).
*   **Chapter 5: Security Posture & Vulnerability Analysis.** (Detail the protection mechanisms against XSS, DoS, and unauthorized access).
*   **Chapter 6: Deployment Strategies.** (Detailed guides on Docker, CI/CD pipelines, Ngrok tunneling, and reverse proxy setups).
