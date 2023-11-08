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
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::{path::PathBuf, sync::Mutex};

#[derive(Debug)]
pub struct SqliteDasDb {
	conn: Mutex<Connection>,
}

impl SqliteDasDb {
	pub fn new(db_path: &str) -> SqliteResult<Self> {
		let conn = Connection::open(db_path)?;
		conn.execute(
			"CREATE TABLE IF NOT EXISTS melodot_das_kvs (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
			[],
		)?;
		Ok(SqliteDasDb { conn: Mutex::new(conn) })
	}
}

impl Default for SqliteDasDb {
	fn default() -> Self {
		let default_path = PathBuf::from("./melodot_light_client.sqlite3");
		Self::new(&default_path.to_str().unwrap()).expect("Default database path should be valid")
	}
}

impl DasKv for SqliteDasDb {
	fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
		let conn = self.conn.lock().unwrap();
		conn.query_row("SELECT value FROM melodot_das_kvs WHERE key = ?", params![key], |row| {
			row.get(0)
		})
		.optional()
		.expect("Should be able to query the database")
	}

	fn set(&mut self, key: &[u8], value: &[u8]) {
		let conn = self.conn.lock().unwrap();
		conn.execute(
			"INSERT OR REPLACE INTO melodot_das_kvs (key, value) VALUES (?,?)",
			params![key, value],
		)
		.expect("Should be able to insert or replace a value in the database");
	}

	fn remove(&mut self, key: &[u8]) {
		let conn = self.conn.lock().unwrap();
		conn.execute("DELETE FROM melodot_das_kvs WHERE key = ?", params![key])
			.expect("Should be able to delete from the database");
	}

	fn contains(&mut self, key: &[u8]) -> bool {
		let conn = self.conn.lock().unwrap();
		let count: i64 = conn
			.query_row("SELECT COUNT(*) FROM melodot_das_kvs WHERE key = ?", params![key], |row| {
				row.get(0)
			})
			.expect("Should be able to count in the database");
		count > 0
	}

	fn compare_and_set(&mut self, key: &[u8], old_value: Option<&[u8]>, new_value: &[u8]) -> bool {
		let conn = self.conn.lock().unwrap();
		match old_value {
			Some(old_val) => {
				let current: Option<Vec<u8>> = conn
					.query_row(
						"SELECT value FROM melodot_das_kvs WHERE key = ?",
						params![key],
						|row| row.get(0),
					)
					.optional()
					.expect("Should be able to query the database");

				if current.as_deref() == Some(old_val) {
					conn.execute(
						"INSERT OR REPLACE INTO melodot_das_kvs (key, value) VALUES (?,?)",
						params![key, new_value],
					)
					.expect("Should be able to insert or replace a value in the database");
					true
				} else {
					false
				}
			},
			None => {
				let count: i64 = conn
					.query_row(
						"SELECT COUNT(*) FROM melodot_das_kvs WHERE key = ?",
						params![key],
						|row| row.get(0),
					)
					.expect("Should be able to count in the database");
				if count == 0 {
					conn.execute(
						"INSERT INTO melodot_das_kvs (key, value) VALUES (?,?)",
						params![key, new_value],
					)
					.expect("Should be able to insert a value in the database");
					true
				} else {
					false
				}
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::tempdir;

	fn setup_test_db() -> SqliteDasDb {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.sqlite3");
		SqliteDasDb::new(db_path.to_str().unwrap()).unwrap()
	}

	#[test]
	fn test_new_db() {
		let _db = setup_test_db();
		// Further assertions can be made to ensure table creation, etc.
	}

	#[test]
	fn test_set_and_get() {
		let mut db = setup_test_db();
		let key = b"test_key";
		let value = b"test_value";

		db.set(key, value);
		assert_eq!(db.get(key), Some(value.to_vec()));
	}

	#[test]
	fn test_remove() {
		let mut db = setup_test_db();
		let key = b"test_key";
		let value = b"test_value";

		db.set(key, value);
		assert!(db.contains(key));

		db.remove(key);
		assert!(!db.contains(key));
	}

	#[test]
	fn test_contains() {
		let mut db = setup_test_db();
		let key = b"test_key";
		let value = b"test_value";

		assert!(!db.contains(key));
		db.set(key, value);
		assert!(db.contains(key));
	}

	#[test]
	fn test_compare_and_set() {
		let mut db = setup_test_db();
		let key = b"test_key";
		let old_value = b"test_value_old";
		let new_value = b"test_value_new";

		// Should return false because the key does not exist yet.
		assert!(!db.compare_and_set(key, Some(old_value), new_value));

		// Set the initial value
		db.set(key, old_value);
		// Should return false because the old value doesn't match the actual value.
		assert!(!db.compare_and_set(key, Some(new_value), new_value));
		// Should return true because the old value matches.
		assert!(db.compare_and_set(key, Some(old_value), new_value));

		// The value should have changed.
		assert_eq!(db.get(key), Some(new_value.to_vec()));

		// Now let's test setting with None as old_value, expecting to do nothing if the key exists.
		assert!(!db.compare_and_set(key, None, old_value));

		// Remove the key and try again, it should now insert the value.
		db.remove(key);
		assert!(db.compare_and_set(key, None, old_value));
		assert_eq!(db.get(key), Some(old_value.to_vec()));
	}
}
