initSidebarItems({"enum":[["SQLError",""],["Value","Owning dynamic type value. Value's type is typically dictated by SQLite (not by the caller)."]],"struct":[["SQLQuery","We want to accumulate values that will later be substituted into a SQL statement execution. This struct encapsulates the generated string and the initial argument list. Additional user-supplied argument bindings, with their placeholders accumulated via `push_bind_param`, will be appended to this argument list."],["SQLiteQueryBuilder","A QueryBuilder that implements SQLite's specific escaping rules."]],"trait":[["QueryBuilder","Gratefully based on Diesel's QueryBuilder trait: https://github.com/diesel-rs/diesel/blob/4885f61b8205f7f3c2cfa03837ed6714831abe6b/diesel/src/query_builder/mod.rs#L56"],["QueryFragment",""]],"type":[["BuildQueryResult",""]]});