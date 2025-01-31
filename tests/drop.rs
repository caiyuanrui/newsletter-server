use sqlx::Row;
use zero2prod::configuration::get_configuration;

#[ignore = "drop"]
#[tokio::test]
async fn test_drop() {
    let config = get_configuration().unwrap();
    let conn = sqlx::MySqlPool::connect_with(config.database.without_db())
        .await
        .unwrap();
    let rows = sqlx::query(
        r#"
        SELECT SCHEMA_NAME
        FROM information_schema.schemata
        WHERE SCHEMA_NAME NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys', 'newsletter');
        "#,
    )
    .fetch_all(&conn)
    .await
    .unwrap();

    for row in rows {
        let db_name: String = row.get("SCHEMA_NAME");

        sqlx::query(&format!("DROP DATABASE `{}`;", db_name))
            .execute(&conn)
            .await
            .unwrap_or_else(|_e| panic!("Failed to drop database: {}", db_name));
    }
}
