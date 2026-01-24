use media_identifier::parse;

fn main() {
    let test_cases = [
        "walle-bttf.iii.720.mkv",
        "Back to the Future III (1990)",
        "unknown.file.mkv",
        "bttf.720.mkv",
    ];
    
    for input in test_cases {
        let parsed = parse(input);
        println!("Input: {}", input);
        println!("  Title: {:?}", parsed.title);
        println!("  Year: {:?}", parsed.year);
        println!("  Confidence: {}", parsed.confidence);
        println!("  MediaType: {:?}", parsed.media_type);
        println!("  Release Group: {:?}", parsed.release_group);
        println!();
    }
}
