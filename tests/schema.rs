mod helpers;

#[sqlx::test]
async fn migrations_create_tables(pool: sqlx::PgPool) {
    // #[sqlx::test] runs ./migrations against the isolated test DB before this body.
    let count: i64 = sqlx::query_scalar(
        "select count(*) from information_schema.tables \
         where table_schema = 'public' and table_name in ('contact','project','task')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 3);
}
