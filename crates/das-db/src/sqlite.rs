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
