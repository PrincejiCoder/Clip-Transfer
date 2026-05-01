use bytesize::ByteSize;
use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::{args::ARGS, pasta::PastaFile, Pasta};

pub fn read_all() -> HashMap<String, Pasta> {
    select_all_from_db()
}

pub fn update_all(pastas: &HashMap<String, Pasta>) {
    rewrite_all_to_db(pastas);
}

pub fn rewrite_all_to_db(pasta_data: &HashMap<String, Pasta>) {
    let conn = Connection::open(format!("{}/database.sqlite", ARGS.data_dir))
        .expect("Failed to open SQLite database!");

    conn.execute(
        "DROP TABLE IF EXISTS pasta;",
        params![],
    )
    .expect("Failed to drop SQLite table for Pasta!");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pasta (
            slug TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            file_name TEXT,
            file_size INTEGER,
            extension TEXT NOT NULL,
            read_only INTEGER NOT NULL,
            private INTEGER NOT NULL,
            allow_edit INTEGER NOT NULL,
            encrypt_server INTEGER NOT NULL,
            encrypt_client INTEGER NOT NULL,
            encrypted_key TEXT,
            created INTEGER NOT NULL,
            expiration INTEGER NOT NULL,
            last_read INTEGER NOT NULL,
            read_count INTEGER NOT NULL,
            burn_after_reads INTEGER NOT NULL,
            attachments TEXT,
            pasta_type TEXT NOT NULL
        );",
        params![],
    )
    .expect("Failed to create SQLite table for Pasta!");

    for pasta in pasta_data.values() {
        conn.execute(
            "INSERT INTO pasta (
                slug,
                content,
                file_name,
                file_size,
                extension,
                read_only,
                private,
                allow_edit,
                encrypt_server,
                encrypt_client,
                encrypted_key,
                created,
                expiration,
                last_read,
                read_count,
                burn_after_reads,
                attachments,
                pasta_type
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                pasta.slug,
                pasta.content,
                pasta.file.as_ref().map_or("", |f| f.name.as_str()),
                pasta.file.as_ref().map_or(0, |f| f.size.as_u64()),
                pasta.extension,
                pasta.readonly as i32,
                pasta.private as i32,
                pasta.allow_edit as i32,
                pasta.encrypt_server as i32,
                pasta.encrypt_client as i32,
                pasta.encrypted_key.as_deref(),
                pasta.created,
                pasta.expiration,
                pasta.last_read,
                pasta.read_count,
                pasta.burn_after_reads,
                serde_json::to_string(&pasta.attachments).unwrap_or("".to_string()),
                pasta.pasta_type,
            ],
        )
        .expect("Failed to insert pasta.");
    }
}

pub fn select_all_from_db() -> HashMap<String, Pasta> {
    let conn = Connection::open(format!("{}/database.sqlite", ARGS.data_dir))
        .expect("Failed to open SQLite database!");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pasta (
            slug TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            file_name TEXT,
            file_size INTEGER,
            extension TEXT NOT NULL,
            read_only INTEGER NOT NULL,
            private INTEGER NOT NULL,
            allow_edit INTEGER NOT NULL,
            encrypt_server INTEGER NOT NULL,
            encrypt_client INTEGER NOT NULL,
            encrypted_key TEXT,
            created INTEGER NOT NULL,
            expiration INTEGER NOT NULL,
            last_read INTEGER NOT NULL,
            read_count INTEGER NOT NULL,
            burn_after_reads INTEGER NOT NULL,
            attachments TEXT,
            pasta_type TEXT NOT NULL
        );",
        params![],
    )
    .expect("Failed to create SQLite table for Pasta!");

    let mut stmt = conn
        .prepare("SELECT * FROM pasta ORDER BY created ASC")
        .expect("Failed to prepare SQL statement to load pastas");

    let pasta_iter = stmt
        .query_map([], |row| {
            Ok(Pasta {
                slug: row.get(0)?,
                content: row.get(1)?,
                file: if let (Some(file_name), Some(file_size)) = (row.get(2)?, row.get(3)?) {
                    let file_size: u64 = file_size;
                    if file_name != "" && file_size != 0 {
                        Some(PastaFile {
                            name: file_name,
                            size: ByteSize::b(file_size),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                },
                extension: row.get(4)?,
                readonly: row.get(5)?,
                private: row.get(6)?,
                allow_edit: row.get(7)?,
                encrypt_server: row.get(8)?,
                encrypt_client: row.get(9)?,
                encrypted_key: row.get(10)?,
                created: row.get(11)?,
                expiration: row.get(12)?,
                last_read: row.get(13)?,
                read_count: row.get(14)?,
                burn_after_reads: row.get(15)?,
                pasta_type: row.get(17)?,
                attachments: match row.get::<_, Option<String>>(16) {
                    Ok(Some(json)) => serde_json::from_str(&json).unwrap_or(None),
                    _ => None,
                },
            })
        })
        .expect("Failed to select Pastas from SQLite database.");

    let mut pastas = HashMap::new();
    for pasta_res in pasta_iter {
        let pasta = pasta_res.expect("Failed to get pasta");
        pastas.insert(pasta.slug.clone(), pasta);
    }
    pastas
}

pub fn insert(pasta: &Pasta) {
    let conn = Connection::open(format!("{}/database.sqlite", ARGS.data_dir))
        .expect("Failed to open SQLite database!");

    conn.execute(
        "INSERT INTO pasta (
                slug,
                content,
                file_name,
                file_size,
                extension,
                read_only,
                private,
                allow_edit,
                encrypt_server,
                encrypt_client,
                encrypted_key,
                created,
                expiration,
                last_read,
                read_count,
                burn_after_reads,
                attachments,
                pasta_type
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            pasta.slug,
            pasta.content,
            pasta.file.as_ref().map_or("", |f| f.name.as_str()),
            pasta.file.as_ref().map_or(0, |f| f.size.as_u64()),
            pasta.extension,
            pasta.readonly as i32,
            pasta.private as i32,
            pasta.allow_edit as i32,
            pasta.encrypt_server as i32,
            pasta.encrypt_client as i32,
            pasta.encrypted_key.as_deref(),
            pasta.created,
            pasta.expiration,
            pasta.last_read,
            pasta.read_count,
            pasta.burn_after_reads,
            serde_json::to_string(&pasta.attachments).unwrap_or("".to_string()),
            pasta.pasta_type,
        ],
    )
    .expect("Failed to insert pasta.");
}

pub fn update(pasta: &Pasta) {
    let conn = Connection::open(format!("{}/database.sqlite", ARGS.data_dir))
        .expect("Failed to open SQLite database!");

    conn.execute(
        "UPDATE pasta SET
            content = ?2,
            file_name = ?3,
            file_size = ?4,
            extension = ?5,
            read_only = ?6,
            private = ?7,
            allow_edit = ?8,
            encrypt_server = ?9,
            encrypt_client = ?10,
            encrypted_key = ?11,
            created = ?12,
            expiration = ?13,
            last_read = ?14,
            read_count = ?15,
            burn_after_reads = ?16,
            attachments = ?17,
            pasta_type = ?18
        WHERE slug = ?1;",
        params![
            pasta.slug,
            pasta.content,
            pasta.file.as_ref().map_or("", |f| f.name.as_str()),
            pasta.file.as_ref().map_or(0, |f| f.size.as_u64()),
            pasta.extension,
            pasta.readonly as i32,
            pasta.private as i32,
            pasta.allow_edit as i32,
            pasta.encrypt_server as i32,
            pasta.encrypt_client as i32,
            pasta.encrypted_key.as_deref(),
            pasta.created,
            pasta.expiration,
            pasta.last_read,
            pasta.read_count,
            pasta.burn_after_reads,
            serde_json::to_string(&pasta.attachments).unwrap_or("".to_string()),
            pasta.pasta_type,
        ],
    )
    .expect("Failed to update pasta.");
}

pub fn delete_by_slug(slug: String) {
    let conn = Connection::open(format!("{}/database.sqlite", ARGS.data_dir))
        .expect("Failed to open SQLite database!");

    conn.execute(
        "DELETE FROM pasta 
        WHERE slug = ?1;",
        params![slug],
    )
    .expect("Failed to delete pasta.");
}
