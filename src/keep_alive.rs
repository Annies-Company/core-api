// Render's free plan puts a service to sleep after 15 minutes with no
// incoming requests, and the next visitor waits ~a minute for it to wake.
// This task calls our own public /health every 10 minutes so it never
// gets that far. The request goes out and back in through Render's front
// door, so it counts as traffic.
//
// Render sets RENDER_EXTERNAL_URL on every web service; locally it isn't
// set, so nothing runs. KEEP_ALIVE=false turns it off on Render too.
//
// The same loop runs a tiny query now and then, because Supabase's free
// plan pauses a database that sees no activity for a week.

use std::time::Duration;

use sqlx::PgPool;

const EVERY: Duration = Duration::from_secs(10 * 60);
// one DB touch every 6 pings = once an hour
const DB_EVERY_N: u32 = 6;

pub fn spawn(pool: PgPool) {
    let Ok(base) = std::env::var("RENDER_EXTERNAL_URL") else {
        return;
    };
    if std::env::var("KEEP_ALIVE").is_ok_and(|v| v == "false") {
        return;
    }
    let url = format!("{}/health", base.trim_end_matches('/'));
    println!("keep-alive: pinging {url} every {}s", EVERY.as_secs());

    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        let mut tick = tokio::time::interval(EVERY);
        tick.tick().await; // the first tick fires at once; the server has only just started
        let mut n: u32 = 0;
        loop {
            tick.tick().await;
            n = n.wrapping_add(1);

            match client.get(&url).send().await {
                Ok(res) if res.status().is_success() => {}
                Ok(res) => eprintln!("keep-alive: /health answered {}", res.status()),
                Err(e) => eprintln!("keep-alive: ping failed: {e}"),
            }

            if n % DB_EVERY_N == 0 {
                if let Err(e) = sqlx::query("SELECT 1").execute(&pool).await {
                    eprintln!("keep-alive: DB ping failed: {e}");
                }
            }
        }
    });
}
