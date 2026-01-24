use media_identifier::{parse, parse_debug, MediaType};
use std::env;
use std::io::{self, BufRead};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Check for flags
    let debug_mode = args.iter().any(|a| a == "--debug" || a == "-d");
    let json_mode = args.iter().any(|a| a == "--json" || a == "-j");
    let stdin_mode = args.iter().any(|a| a == "--stdin" || a == "-");
    
    // Filter out flags to get filenames
    let filenames: Vec<&String> = args.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    if stdin_mode || filenames.is_empty() {
        // Read from stdin
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if let Ok(filename) = line {
                let filename = filename.trim();
                if !filename.is_empty() {
                    process_file(filename, debug_mode, json_mode);
                }
            }
        }
    } else {
        // Process command line arguments
        for filename in filenames {
            process_file(filename, debug_mode, json_mode);
        }
    }
}

fn process_file(filename: &str, debug_mode: bool, json_mode: bool) {
    let result = if debug_mode {
        parse_debug(filename)
    } else {
        parse(filename)
    };

    if json_mode {
        // JSON output
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("JSON error: {}", e),
        }
    } else {
        // Human-readable output
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📁 {}", filename);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        let type_icon = match result.media_type {
            MediaType::Movie => "🎬",
            MediaType::Episode => "📺",
            MediaType::Unknown => "❓",
        };
        
        println!("Type:         {} {:?}", type_icon, result.media_type);
        
        if let Some(ref title) = result.title {
            println!("Title:        {}", title);
        }
        
        if let Some(year) = result.year {
            println!("Year:         {}", year);
        }
        
        // Episode info
        if result.media_type == MediaType::Episode {
            if let Some(s) = result.episode_info.season {
                print!("Season:       {}", s);
                if let Some(e) = result.episode_info.episode {
                    print!("  Episode: {}", e);
                    if let Some(ee) = result.episode_info.episode_end {
                        print!("-{}", ee);
                    }
                }
                println!();
            }
            if let Some(ref ep_title) = result.episode_info.episode_title {
                println!("Ep. Title:    {}", ep_title);
            }
        }
        
        // Quality info
        let quality_parts: Vec<String> = [
            result.quality.resolution.clone(),
            result.quality.source.clone(),
            result.quality.codec.clone(),
            result.quality.audio.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        
        if !quality_parts.is_empty() {
            println!("Quality:      {}", quality_parts.join(" | "));
        }
        
        if !result.languages.is_empty() {
            println!("Languages:    {}", result.languages.join(", "));
        }
        
        if let Some(ref group) = result.release_group {
            println!("Group:        {}", group);
        }
        
        if let Some(ref container) = result.container {
            println!("Container:    {}", container);
        }
        
        println!("Confidence:   {}%", result.confidence);
        
        // Debug info
        if debug_mode && !result.matches.is_empty() {
            println!("\n📍 Matches:");
            for m in &result.matches {
                println!("   {:?}[{}..{}] = '{}'", m.category, m.start, m.end, m.value);
            }
        }
        
        println!();
    }
}
