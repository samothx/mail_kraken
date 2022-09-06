use anyhow::Result;
use mysql::{params, prelude::Queryable, Pool};

const ST_DROP_TABLE: &str = r#"drop table if exists test"#;
const ST_CREATE_TABLE: &str = r#"CREATE TABLE `test` (
     `id` bigint unsigned NOT NULL AUTO_INCREMENT,
     `test` varchar(512) NOT NULL,
     PRIMARY KEY (`id`),
     UNIQUE (test)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
"#;
const ST_INSERT: &str = r#"insert into test (test) values(:test)"#;
// const ST_SELECT: &str = r#"select id, test from test order by test asc"#;

fn main() -> Result<()> {
    let pool = Pool::new("mysql://mysql-test:geheim4711@localhost:3306/mysql_test")?;
    let mut db_conn = pool.get_conn()?;

    println!("dropping table");
    db_conn.query_drop(ST_DROP_TABLE)?;
    println!("creating table");
    db_conn.query_drop(ST_CREATE_TABLE)?;

    let strings = vec!["str1", "str2", "str3", "str4", "str5", "str6"];
    println!("inserting strings");
    db_conn.exec_batch(ST_INSERT, strings.iter().map(|val| params! {"test"=>*val}))?;

    Ok(())
}
