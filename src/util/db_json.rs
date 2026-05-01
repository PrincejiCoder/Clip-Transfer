use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter};
use std::collections::HashMap;

use crate::Pasta;

static DATABASE_PATH: &str = "pasta_data/database.json";

pub fn read_all() -> HashMap<String, Pasta> {
    load_from_file().expect("Failed to load pastas from JSON")
}

pub fn update_all(pastas: &HashMap<String, Pasta>) {
    save_to_file(pastas);
}

fn save_to_file(pasta_data: &HashMap<String, Pasta>) {
    let tmp_file_path = DATABASE_PATH.to_string() + ".tmp";
    let tmp_file = File::create(&tmp_file_path).expect(&format!(
        "failed to create temporary database file for writing. path: {tmp_file_path}"
    ));

    let writer = BufWriter::new(tmp_file);
    serde_json::to_writer(writer, &pasta_data)
        .expect("Should be able to write out data to database file");
    std::fs::rename(tmp_file_path, DATABASE_PATH).expect("Could not update database");
}

fn load_from_file() -> io::Result<HashMap<String, Pasta>> {
    let file = File::open(DATABASE_PATH);
    match file {
        Ok(_) => {
            let reader = BufReader::new(file.unwrap());
            let data: HashMap<String, Pasta> = match serde_json::from_reader(reader) {
                Ok(t) => t,
                _ => HashMap::new(),
            };
            Ok(data)
        }
        Err(_) => {
            log::info!("Database file {} not found!", DATABASE_PATH);
            save_to_file(&HashMap::<String, Pasta>::new());

            log::info!("Database file {} created.", DATABASE_PATH);
            load_from_file()
        }
    }
}
