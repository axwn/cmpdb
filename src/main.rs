use chrono::{DateTime, Utc};
use clap::Parser;
use sqlx::postgres::PgPool;
use std::process::ExitCode;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Debug, sqlx::FromRow)]
struct DateRow {
    _date: DateTime<Utc>,
    _count: i64,
    _id: i64,
    _tmstmp: i64,
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
}

async fn run_query(pool: &PgPool) -> Vec<DateRow> {
    let mut conn = pool.acquire().await.unwrap();
    let rows = sqlx::query_as::<_, DateRow>(
        r#"
SELECT
  date_trunc('day', tmstmp) AS _date,
  COUNT(*) AS _count,
  SUM(id % 10)::bigint AS _id,
  SUM(EXTRACT(EPOCH FROM tmstmp)::bigint)::bigint AS _tmstmp
FROM
  predictions
GROUP BY
  _date
ORDER BY
  _date;
"#,
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();

    rows
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let db_a_conn = args.database_a.clone();
    let db_b_conn = args.database_b.clone();

    let task_a = tokio::spawn(async move {
        let db_a_pool = PgPool::connect(&db_a_conn).await.unwrap();
        let rows = run_query(&db_a_pool).await;
        println!("Retrieved {} rows from database A", rows.len());
        rows
    });
    let task_b = tokio::spawn(async move {
        let db_b_pool = PgPool::connect(&db_b_conn).await.unwrap();
        let rows = run_query(&db_b_pool).await;
        println!("Retrieved {} rows from database B", rows.len());
        rows
    });
    let (db_a_rows, db_b_rows) = (task_a.await.unwrap(), task_b.await.unwrap());

    let mut misses = 0;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    for (a, b) in db_a_rows.iter().zip(db_b_rows.iter()) {
        if a._count == b._count && a._id == b._id && a._tmstmp == b._tmstmp {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                .unwrap();
            print!("\u{2714}");
        } else {
            misses += 1;
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
                .unwrap();
            print!("\u{2718}");
        }

        stdout.reset().unwrap();
        print!(" {}", a._date);
        println!(
            " a=({},{},{}) b=({},{},{})",
            a._count, a._id, a._tmstmp, b._count, b._id, b._tmstmp
        );
    }

    return match misses == 0 {
        true => ExitCode::SUCCESS,
        false => {
            eprintln!("Databases do not match");
            ExitCode::FAILURE
        }
    };
}
