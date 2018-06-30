// Copyright 2018 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

#![allow(dead_code)]

use rusqlite;
use uuid::Uuid;

use schema;
use errors::{
    TolstoyError,
    Result,
};

use mentat_core::{
    Entid,
};

use mentat_db::{
    Partition,
    PartitionMap,
};

pub struct SyncMetadataClient {}

pub enum PartitionsTable {
    Core,
    Tolstoy,
}

impl SyncMetadataClient {
    pub fn remote_head(tx: &rusqlite::Transaction) -> Result<Uuid> {
        tx.query_row(
            "SELECT value FROM tolstoy_metadata WHERE key = ?",
            &[&schema::REMOTE_HEAD_KEY], |r| {
                let bytes: Vec<u8> = r.get(0);
                Uuid::from_bytes(bytes.as_slice())
            }
        )?.map_err(|e| e.into())
    }

    pub fn set_remote_head(tx: &rusqlite::Transaction, uuid: &Uuid) -> Result<()> {
        let uuid_bytes = uuid.as_bytes().to_vec();
        let updated = tx.execute("UPDATE tolstoy_metadata SET value = ? WHERE key = ?",
            &[&uuid_bytes, &schema::REMOTE_HEAD_KEY])?;
        if updated != 1 {
            bail!(TolstoyError::DuplicateMetadata(schema::REMOTE_HEAD_KEY.into()));
        }
        Ok(())
    }

    // TODO Functions below start to blur the line between mentat-proper and tolstoy...
    pub fn get_partitions(tx: &rusqlite::Transaction, parts_table: PartitionsTable) -> Result<PartitionMap> {
        let db_table = match parts_table {
            PartitionsTable::Core => "parts",
            PartitionsTable::Tolstoy => "tolstoy_parts"
        };
        let mut stmt: ::rusqlite::Statement = tx.prepare(&format!("SELECT part, start, end, idx FROM {}", db_table))?;
        let m: Result<PartitionMap> = stmt.query_and_then(&[], |row| -> Result<(String, Partition)> {
            Ok((row.get_checked(0)?, Partition::new(row.get_checked(1)?, row.get_checked(2)?, row.get_checked(3)?)))
        })?.collect();
        m
    }

    pub fn root_and_head_tx(tx: &rusqlite::Transaction) -> Result<(Entid, Entid)> {
        let mut stmt: ::rusqlite::Statement = tx.prepare("SELECT tx FROM transactions GROUP BY tx ORDER BY tx")?;
        let txs: Vec<_> = stmt.query_and_then(&[], |row| -> Result<Entid> {
            Ok(row.get_checked(0)?)
        })?.collect();

        let mut txs = txs.into_iter();

        let root_tx = match txs.nth(0) {
            None => bail!(TolstoyError::UnexpectedState(format!("Could not get root tx"))),
            Some(t) => t?
        };

        match txs.last() {
            None => Ok((root_tx, root_tx)),
            Some(t) => Ok((root_tx, t?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mentat_db::db;

    #[test]
    fn test_get_remote_head_default() {
        let mut conn = schema::tests::setup_conn_bare();
        let tx = schema::tests::setup_tx(&mut conn);
        assert_eq!(Uuid::nil(), SyncMetadataClient::remote_head(&tx).expect("fetch succeeded"));
    }

    #[test]
    fn test_set_and_get_remote_head() {
        let mut conn = schema::tests::setup_conn_bare();
        let tx = schema::tests::setup_tx(&mut conn);
        let uuid = Uuid::new_v4();
        SyncMetadataClient::set_remote_head(&tx, &uuid).expect("update succeeded");
        assert_eq!(uuid, SyncMetadataClient::remote_head(&tx).expect("fetch succeeded"));
    }

    #[test]
    fn test_root_and_head_tx() {
        let mut conn = schema::tests::setup_conn_bare();
        db::ensure_current_version(&mut conn).expect("mentat db init");
        let db_tx = conn.transaction().expect("transaction");

        let (root_tx, last_tx) = SyncMetadataClient::root_and_head_tx(&db_tx).expect("last tx");
        assert_eq!(268435456, root_tx);
        assert_eq!(268435456, last_tx);

        // These are determenistic, but brittle.
        // Inserting a tx 268435457 at time 1529971773701734
        // 268435457|3|1529971773701734|268435457|1|4
        // ... which defines entity ':person/name'...
        // 65536|1|:person/name|268435457|1|13
        // ... which has valueType of string
        // 65536|7|27|268435457|1|0
        // ... which is unique...
        // 65536|9|36|268435457|1|0
        // ... ident
        // 65536|11|1|268435457|1|1

        db_tx.execute("INSERT INTO transactions VALUES (?, ?, ?, ?, ?, ?)", &[&268435457, &3, &1529971773701734_i64, &268435457, &1, &4]).expect("inserted");
        db_tx.execute("INSERT INTO transactions VALUES (?, ?, ?, ?, ?, ?)", &[&65536, &1, &":person/name", &268435457, &1, &13]).expect("inserted");
        db_tx.execute("INSERT INTO transactions VALUES (?, ?, ?, ?, ?, ?)", &[&65536, &7, &27, &268435457, &1, &0]).expect("inserted");
        db_tx.execute("INSERT INTO transactions VALUES (?, ?, ?, ?, ?, ?)", &[&65536, &9, &36, &268435457, &1, &0]).expect("inserted");
        db_tx.execute("INSERT INTO transactions VALUES (?, ?, ?, ?, ?, ?)", &[&65536, &11, &1, &268435457, &1, &1]).expect("inserted");

        let (root_tx, last_tx) = SyncMetadataClient::root_and_head_tx(&db_tx).expect("last tx");
        assert_eq!(268435456, root_tx);
        assert_eq!(268435457, last_tx);
    }
}
