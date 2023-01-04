use clap::Parser;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::process::ExitCode;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Debug, sqlx::FromRow)]
struct DateRow {
    _date: time::OffsetDateTime,
    _count: i64,
    _id: rust_decimal::Decimal,
    _tmstmp: rust_decimal::Decimal,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        long,
        value_name = "CONNECTION_STRING",
        help = "PostgreSQL connection string for database A",
        required = true
    )]
    database_a: String,
    #[arg(
        long,
        value_name = "CONNECTION_STRING",
        help = "PostgreSQL connection string for database B",
        required = true
    )]
    database_b: String,
    #[arg(
        long,
        value_name = "TABLE",
        help = "Table in database A",
        default_value = "predictions"
    )]
    table_a: String,
    #[arg(
        long,
        value_name = "TABLE",
        help = "Table in database B",
        default_value = "predictions"
    )]
    table_b: String,
    #[arg(
        long,
        value_name = "DATE",
        help = "First day (YYYYMMDD)",
        required = true
    )]
    first_day: String,
    #[arg(
        long,
        value_name = "DATE",
        help = "Last day (YYYYMMDD)",
        required = true
    )]
    last_day: String,
}

async fn run_query(pool: &PgPool, table: &str) -> Vec<DateRow> {
    let args = Args::parse();
    let first_day = args.first_day.replace(|c: char| !c.is_ascii_digit(), "");
    let last_day = args.last_day.replace(|c: char| !c.is_ascii_digit(), "");
    let query = format!(
        r#"
SELECT
  date_trunc('day', tmstmp) AS _date,
  COUNT(*) AS _count,
  SUM(id)::NUMERIC AS _id,
  SUM(EXTRACT(EPOCH FROM tmstmp))::NUMERIC AS _tmstmp
FROM
  {}
WHERE
  tmstmp >= '{}'::date
AND
  tmstmp < ('{}'::date + INTERVAL '24 hour')
GROUP BY
  _date
ORDER BY
  _date;
"#,
        table, first_day, last_day
    );

    let mut conn = pool.acquire().await.unwrap();
    let rows = sqlx::query_as::<_, DateRow>(&query)
        .fetch_all(&mut conn)
        .await
        .unwrap();

    rows
}

fn green_check(stdout: &mut StandardStream) {
    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
        .unwrap();
    print!("\u{2714}");
}

fn red_x(stdout: &mut StandardStream) {
    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
        .unwrap();
    print!("\u{2718}");
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let db_a_conn = args.database_a.clone();
    let db_b_conn = args.database_b.clone();
    let db_table_a = args.table_a.replace(|c: char| !c.is_ascii_alphabetic(), "");
    let db_table_b = args.table_b.replace(|c: char| !c.is_ascii_alphabetic(), "");

    let task_a = tokio::spawn(async move {
        let db_a_pool = PgPool::connect(&db_a_conn).await.unwrap();
        let rows = run_query(&db_a_pool, &db_table_a).await;
        println!("Retrieved {} rows from database A", rows.len());
        rows
    });
    let task_b = tokio::spawn(async move {
        let db_b_pool = PgPool::connect(&db_b_conn).await.unwrap();
        let rows = run_query(&db_b_pool, &db_table_b).await;
        println!("Retrieved {} rows from database B", rows.len());
        rows
    });
    let (db_a_rows, db_b_rows) = (task_a.await.unwrap(), task_b.await.unwrap());

    let mut map_a = HashMap::new();
    let mut map_b = HashMap::new();

    for a in db_a_rows.iter() {
        map_a.insert(a._date, a);
    }

    for b in db_b_rows.iter() {
        map_b.insert(b._date, b);
    }

    let mut shared = 0;
    let mut matches = 0;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    for (k1, v1) in map_a.iter() {
        match map_b.get(k1) {
            Some(v2) => {
                shared += 1;
                if v1._count == v2._count && v1._id == v2._id && v1._tmstmp == v2._tmstmp {
                    matches += 1;
                    green_check(&mut stdout);
                } else {
                    red_x(&mut stdout);
                }
            }
            None => {
                red_x(&mut stdout);
            }
        }
    }
    stdout.reset().unwrap();
    println!();

    return match db_a_rows.len() == db_b_rows.len() && shared == matches {
        true => {
            println!("Databases match!");
            ExitCode::SUCCESS
        }
        false => {
            eprintln!("Databases do not match.");
            ExitCode::FAILURE
        }
    };
}
