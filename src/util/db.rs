use std::collections::HashMap;
use crate::{args::ARGS, pasta::Pasta};

#[cfg(not(feature = "default"))]
const PANIC_MSG: &'static str = "Can not run without argument json-db, this version of microbin was compiled without rusqlite support. Make sure you do not pass in no-default-features during compilation";

pub fn read_all() -> HashMap<String, Pasta> {
    if ARGS.json_db {
        super::db_json::read_all()
    } else {
        #[cfg(feature = "default")]
        { super::db_sqlite::read_all() }
        #[cfg(not(feature = "default"))]
        { panic!("{}", PANIC_MSG); }
    }
}

pub fn insert(pastas: &HashMap<String, Pasta>, pasta: &Pasta) {
    if ARGS.json_db {
        super::db_json::update_all(pastas);
    } else {
        #[cfg(feature = "default")]
        { super::db_sqlite::insert(pasta); }
        #[cfg(not(feature = "default"))]
        { panic!("{}", PANIC_MSG); }
    }
}

pub fn update(pastas: &HashMap<String, Pasta>, pasta: &Pasta) {
    if ARGS.json_db {
        super::db_json::update_all(pastas);
    } else {
        #[cfg(feature = "default")]
        { super::db_sqlite::update(pasta); }
        #[cfg(not(feature = "default"))]
        { panic!("{}", PANIC_MSG); }
    }
}

pub fn update_all(pastas: &HashMap<String, Pasta>) {
    if ARGS.json_db {
        super::db_json::update_all(pastas);
    } else {
        #[cfg(feature = "default")]
        { super::db_sqlite::update_all(pastas); }
        #[cfg(not(feature = "default"))]
        { panic!("{}", PANIC_MSG); }
    }
}

pub fn delete(pastas: &HashMap<String, Pasta>, slug: &str) {
    if ARGS.json_db {
        super::db_json::update_all(pastas);
    } else {
        #[cfg(feature = "default")]
        { super::db_sqlite::delete_by_slug(slug.to_string()); }
        #[cfg(not(feature = "default"))]
        { panic!("{}", PANIC_MSG); }
    }
}
