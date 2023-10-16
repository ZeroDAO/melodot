// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::traits::DasKv;
use rusqlite::{self, OptionalExtension};

use rusqlite::{params, Connection};
use std::{
	path::PathBuf,
	sync::{Arc, Mutex},
};

pub struct SqliteDasDb {
	conn: Arc<Mutex<Connection>>,
}

impl SqliteDasDb {
	pub fn new(db_path: &str) -> rusqlite::Result<Self> {
		let conn = Connection::open(db_path)?;
		conn.execute(
			"CREATE TABLE IF NOT EXISTS kvs (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
			[],
		)?;
		Ok(Self { conn: Arc::new(Mutex::new(conn)) })
	}
}

impl Default for SqliteDasDb {
	fn default() -> Self {
		let default_path = PathBuf::from("./default_db.sqlite3");
		Self::new(&default_path.to_str().unwrap()).unwrap()
	}
}

impl DasKv for SqliteDasDb {
	fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
		let conn = self.conn.lock().unwrap();
		conn.query_row("SELECT value FROM kvs WHERE key = ?", params![key], |row| row.get(0))
			.optional()
			.unwrap_or(None)
	}

	fn set(&mut self, key: &[u8], value: &[u8]) {
		let conn = self.conn.lock().unwrap();
		conn.execute("INSERT OR REPLACE INTO kvs (key, value) VALUES (?,?)", params![key, value])
			.unwrap();
	}

	fn remove(&mut self, key: &[u8]) {
		let conn = self.conn.lock().unwrap();
		conn.execute("DELETE FROM kvs WHERE key = ?", params![key]).unwrap();
	}

	fn contains(&mut self, key: &[u8]) -> bool {
		let conn = self.conn.lock().unwrap();
		let count: i64 = conn
			.query_row("SELECT COUNT(*) FROM kvs WHERE key = ?", params![key], |row| row.get(0))
			.unwrap();
		count > 0
	}

    fn compare_and_set(&mut self, key: &[u8], old_value: Option<&[u8]>, new_value: &[u8]) -> bool {
        let conn = self.conn.lock().unwrap();
        match old_value {
            Some(old_val) => {
                // Check if the current value matches the old value.
                let current: Option<Vec<u8>> = conn.query_row("SELECT value FROM kvs WHERE key = ?", params![key], |row| row.get(0))
                    .optional()
                    .unwrap_or(None);
                if current.as_ref() == Some(&old_val.to_vec()) {
                    // If they match, update to the new value.
                    conn.execute("INSERT OR REPLACE INTO kvs (key, value) VALUES (?,?)", params![key, new_value])
                        .unwrap();
                    true
                } else {
                    false
                }
            },
            None => {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM kvs WHERE key = ?", params![key], |row| row.get(0))
                    .unwrap();
                if count == 0 {
                    conn.execute("INSERT OR REPLACE INTO kvs (key, value) VALUES (?,?)", params![key, new_value])
                        .unwrap();
                    true
                } else {
                    false
                }
            },
        }
    }
    
}
