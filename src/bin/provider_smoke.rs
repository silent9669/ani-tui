use ani_tui::config::Config;
use ani_tui::providers::ProviderRegistry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = ProviderRegistry::new(&Config::default());
    println!(
        "Providers: {:?}",
        registry
            .list_providers()
            .iter()
            .map(|p| p.name())
            .collect::<Vec<_>>()
    );

    for provider in registry.list_providers() {
        let name = provider.name().to_string();
        println!("\n=== {} ===", name);
        let results = match provider.search("one piece").await {
            Ok(r) => r,
            Err(e) => {
                println!("  SEARCH FAILED: {e}");
                continue;
            }
        };
        if results.is_empty() {
            println!("  SEARCH: no results");
            continue;
        }
        let top = &results[0];
        println!(
            "  TOP: {} ({} eps) {}",
            top.title,
            top.total_episodes.unwrap_or(0),
            top.id
        );

        let episodes = match provider.get_episodes(&top.id).await {
            Ok(e) => e,
            Err(e) => {
                println!("  EPISODES FAILED: {e}");
                continue;
            }
        };
        println!("  EPISODES: {}", episodes.len());
        if episodes.is_empty() {
            continue;
        }
        let first = &episodes[0];
        let last = episodes.last().unwrap();
        println!(
            "  FIRST EP: {} ({})  LAST EP: {} ({})",
            first.number, first.id, last.number, last.id
        );

        let target = if episodes.len() > 10 {
            episodes[episodes.len() / 2].clone()
        } else {
            first.clone()
        };
        match provider.get_stream_url(&target.id).await {
            Ok(stream) => {
                println!(
                    "  STREAM OK: {} subtitles={} qualities={:?} use_curl={}",
                    stream.video_url,
                    stream.subtitles.len(),
                    stream.qualities,
                    stream.use_curl
                );
                for s in &stream.subtitles {
                    println!("    SUB: {} -> {}", s.language, s.url);
                }
            }
            Err(e) => println!("  STREAM FAILED: {e}"),
        }
    }
    Ok(())
}
