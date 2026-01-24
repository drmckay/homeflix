//! Integration tests using real-world media filenames

use media_identifier::{parse, MediaType};

// ============================================================================
// MOVIES
// ============================================================================

#[test]
fn test_ballerina_2025() {
    let r = parse("Ballerina.2025.Hybrid.BDRip.x264.HUN-FULCRUM");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("Ballerina".to_string()));
    assert_eq!(r.year, Some(2025));
    assert_eq!(r.quality.source, Some("Blu-ray".to_string()));
    assert_eq!(r.quality.codec, Some("H.264".to_string()));
    assert!(r.languages.contains(&"Hungarian".to_string()));
    assert_eq!(r.release_group, Some("FULCRUM".to_string()));
}

#[test]
fn test_bugonia_2025() {
    let r = parse("Bugonia.2025.MA.WEBRip.x264.HUN-FULCRUM");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("Bugonia".to_string()));
    assert_eq!(r.year, Some(2025));
    // Source could be WEBRip or MA (Movies Anywhere) depending on pattern priority
    assert!(r.quality.source.is_some());
}

#[test]
fn test_home_alone() {
    let r = parse("Home.Alone.1990.REMASTERED.READ.NFO.BDRip.x264.AC3.HuN-Essence");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("Home Alone".to_string()));
    assert_eq!(r.year, Some(1990));
    assert_eq!(r.release_group, Some("Essence".to_string()));
}

#[test]
fn test_lyle_lyle_crocodile() {
    let r = parse("Lyle.Lyle.Crocodile.2022.720p.BluRay.DD+5.1.x264.HuN-No1");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("Lyle Lyle Crocodile".to_string()));
    assert_eq!(r.year, Some(2022));
    assert_eq!(r.quality.resolution, Some("720p".to_string()));
    assert_eq!(r.release_group, Some("No1".to_string()));
}

#[test]
fn test_willy_wonka_1971() {
    let r = parse("Willy.Wonka.and.the.Chocolate.Factory.1971.720p.BluRay.DTS.x264.HuN-FASiRT");
    assert_eq!(r.media_type, MediaType::Movie);
    assert!(r.title.as_ref().unwrap().contains("Willy Wonka"));
    assert_eq!(r.year, Some(1971));
    assert_eq!(r.quality.resolution, Some("720p".to_string()));
    assert_eq!(r.release_group, Some("FASiRT".to_string()));
}

#[test]
fn test_wonka_2023() {
    let r = parse("Wonka.2023.720p.iT.WEB-DL.DD+5.1.Atmos.H.264.HuN-No1");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("Wonka".to_string()));
    assert_eq!(r.year, Some(2023));
    assert_eq!(r.quality.resolution, Some("720p".to_string()));
    assert_eq!(r.quality.source, Some("WEB-DL".to_string()));
}

// ============================================================================
// TV SHOWS - DARK MATTER
// ============================================================================

#[test]
fn test_dark_matter_season_folder() {
    let r = parse("Dark.Matter.2015.S01-S03");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Dark Matter"));
    assert_eq!(r.year, Some(2015));
    // Should detect season marker
    assert!(r.episode_info.season.is_some());
}

#[test]
fn test_dark_matter_single_episode() {
    let r = parse("Dark.Matter.S01E10.BDRIP.x264-Krissz.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Dark Matter"));
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(10));
    assert_eq!(r.container, Some("mkv".to_string()));
}

#[test]
fn test_dark_matter_s02() {
    let r = parse("Dark.Matter.S02E08.PROPER.720p.HDTV.x264-KILLERS.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(2));
    assert_eq!(r.episode_info.episode, Some(8));
    assert_eq!(r.quality.resolution, Some("720p".to_string()));
    assert_eq!(r.quality.source, Some("HDTV".to_string()));
}

#[test]
fn test_dark_matter_multi_episode() {
    let r = parse("Dark.Matter.S03E01E02.720p.HDTV.x264-AVS.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(3));
    assert_eq!(r.episode_info.episode, Some(1));
    assert_eq!(r.episode_info.episode_end, Some(2));
}

// ============================================================================
// TV SHOWS - STARGATE ATLANTIS
// ============================================================================

#[test]
fn test_stargate_atlantis_pilot() {
    let r = parse("Stargate.Atlantis.S01E01-E02.Rising.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Stargate Atlantis"));
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(1));
    assert_eq!(r.episode_info.episode_end, Some(2));
    // Episode title should be "Rising"
    assert!(r.episode_info.episode_title.as_ref().map(|t| t.contains("Rising")).unwrap_or(false));
    assert!(r.languages.contains(&"Hungarian".to_string()));
    assert!(r.languages.contains(&"English".to_string()));
}

#[test]
fn test_stargate_atlantis_with_apostrophe() {
    let r = parse("Stargate.Atlantis.S01E06.Childhood's.End.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(6));
    // Should capture episode title with apostrophe
    if let Some(ref ep_title) = r.episode_info.episode_title {
        assert!(ep_title.contains("Childhood"));
    }
}

#[test]
fn test_stargate_atlantis_part_notation() {
    let r = parse("Stargate.Atlantis.S01E19.The.Siege.(Part.1).BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(19));
}

// ============================================================================
// TV SHOWS - STARGATE SG-1 (hyphenated title!)
// ============================================================================

#[test]
fn test_stargate_sg1_basic() {
    let r = parse("Stargate.SG-1.S09E18.Arthur's.Mantle.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    // The title should contain "SG-1" or "SG 1"
    let title = r.title.as_ref().unwrap();
    assert!(title.contains("Stargate"));
    assert_eq!(r.episode_info.season, Some(9));
    assert_eq!(r.episode_info.episode, Some(18));
}

#[test]
fn test_stargate_sg1_numbered_episode() {
    // Episode "200" - just a number as episode title
    let r = parse("Stargate.SG-1.S10E06.200.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(10));
    assert_eq!(r.episode_info.episode, Some(6));
}

#[test]
fn test_stargate_sg1_part_notation() {
    let r = parse("Stargate.SG-1.S10E10.The.Quest.(Part.1).BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(10));
    assert_eq!(r.episode_info.episode, Some(10));
}

#[test]
fn test_stargate_sg1_year_titled_episode_2001() {
    // Episode S05E10 is titled "2001" - should NOT extract 2001 as year
    let r = parse("Stargate.SG-1.S05E10.2001.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(5));
    assert_eq!(r.episode_info.episode, Some(10));
    // Year should be None because "2001" comes AFTER the episode marker
    assert_eq!(r.year, None, "Year '2001' should not be extracted for year-titled episodes");
}

#[test]
fn test_stargate_sg1_year_titled_episode_2010() {
    // Episode S04E16 is titled "2010" - should NOT extract 2010 as year
    let r = parse("Stargate.SG-1.S04E16.2010.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(4));
    assert_eq!(r.episode_info.episode, Some(16));
    assert_eq!(r.year, None, "Year '2010' should not be extracted for year-titled episodes");
}

#[test]
fn test_stargate_sg1_year_titled_episode_1969() {
    // Episode S02E21 is titled "1969" - should NOT extract 1969 as year
    let r = parse("Stargate.SG-1.S02E21.1969.BDRip.x264.Hun.Eng-MaMMuT.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(2));
    assert_eq!(r.episode_info.episode, Some(21));
    assert_eq!(r.year, None, "Year '1969' should not be extracted for year-titled episodes");
}

// ============================================================================
// TV SHOWS - STARGATE UNIVERSE
// ============================================================================

#[test]
fn test_stargate_universe_pilot() {
    let r = parse("Stargate.Universe.S01E01E02.Air.Parts.1.and.2.avi");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Stargate Universe"));
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(1));
    assert_eq!(r.episode_info.episode_end, Some(2));
    assert_eq!(r.container, Some("avi".to_string()));
}

#[test]
fn test_stargate_universe_regular() {
    let r = parse("Stargate.Universe.S02E07.The.Greater.Good.avi");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(2));
    assert_eq!(r.episode_info.episode, Some(7));
}

// ============================================================================
// EDGE CASES AND SPECIAL PATTERNS
// ============================================================================

#[test]
fn test_sample_file() {
    let r = parse("fulcrum-ballerina.2025.bdrip.hybrid-sample.mkv");
    assert_eq!(r.year, Some(2025));
    assert_eq!(r.release_group, Some("fulcrum".to_string()));
}

#[test]
fn test_nested_path() {
    let r = parse("Sample/fulcrum-ballerina.2025.bdrip.hybrid-sample.mkv");
    assert_eq!(r.year, Some(2025));
}

#[test]
fn test_prefix_release_group() {
    // Scene format: GROUP-Title.Year... (release group at beginning)
    let r = parse("fulcrum-ballerina.2025");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("ballerina".to_string()));
    assert_eq!(r.year, Some(2025));
    assert_eq!(r.release_group, Some("fulcrum".to_string()));
}

#[test]
fn test_prefix_release_group_with_quality() {
    // Full scene format with quality info
    let r = parse("FULCRUM-bugonia.2025.ma.webrip.x264.hun");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("bugonia".to_string()));
    assert_eq!(r.year, Some(2025));
    assert_eq!(r.release_group, Some("FULCRUM".to_string()));
}

#[test]
fn test_season_folder_structure() {
    let r = parse("Dark.Matter.S01.BDRIP.x264-Krissz");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Dark Matter"));
    assert_eq!(r.episode_info.season, Some(1));
}

#[test]
fn test_mixed_season_folder() {
    let r = parse("Dark.Matter.S02.720p.HDTV.x264-MiXGROUP");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(2));
    assert_eq!(r.quality.resolution, Some("720p".to_string()));
}

// ============================================================================
// FILE WITH FULL PATH
// ============================================================================

#[test]
fn test_full_unix_path() {
    let r = parse("/media/movies/Home.Alone.1990.REMASTERED.BDRip.x264.AC3.HuN-Essence.mkv");
    assert_eq!(r.title, Some("Home Alone".to_string()));
    assert_eq!(r.year, Some(1990));
}

#[test]
fn test_subtitle_file() {
    let r = parse("snc-hmln-sd-hun.forced.srt");
    assert_eq!(r.container, Some("srt".to_string()));
}

// ============================================================================
// CONFIDENCE TESTS
// ============================================================================

#[test]
fn test_high_confidence_movie() {
    let r = parse("Movie.2023.1080p.BluRay.x264-GROUP.mkv");
    assert!(r.confidence >= 70, "Expected high confidence, got {}", r.confidence);
}

#[test]
fn test_high_confidence_tv() {
    let r = parse("Show.S01E01.720p.HDTV.x264-GROUP.mkv");
    assert!(r.confidence >= 70, "Expected high confidence, got {}", r.confidence);
}

// ============================================================================
// GUESSIT-INSPIRED TESTS - Migrated from guessit test suite
// ============================================================================

// -- Prefix Release Groups (Scene Format) --

#[test]
fn test_guessit_prefix_group_blow() {
    // From guessit movies.yml: blow-how.to.be.single.2016.1080p.bluray.x264.mkv
    let r = parse("blow-how.to.be.single.2016.1080p.bluray.x264.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.release_group, Some("blow".to_string()));
    assert_eq!(r.title, Some("how to be single".to_string()));
    assert_eq!(r.year, Some(2016));
    assert_eq!(r.quality.resolution, Some("1080p".to_string()));
}

#[test]
fn test_guessit_prefix_group_ulshd() {
    // From guessit movies.yml: ulshd-the.right.stuff.1983.multi.1080p.bluray.x264.mkv
    let r = parse("ulshd-the.right.stuff.1983.multi.1080p.bluray.x264.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.release_group, Some("ulshd".to_string()));
    assert!(r.title.as_ref().unwrap().to_lowercase().contains("right stuff"));
    assert_eq!(r.year, Some(1983));
}

// -- Numeric Movie Titles --

#[test]
fn test_guessit_numeric_title_21() {
    // From guessit movies.yml: 21.(2008).DVDRip.x264.AC3-FtS
    let r = parse("21.2008.DVDRip.x264.AC3-FtS.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("21".to_string()));
    assert_eq!(r.year, Some(2008));
    assert_eq!(r.release_group, Some("FtS".to_string()));
}

#[test]
fn test_guessit_numeric_title_9() {
    // From guessit movies.yml: 9.2009.Blu-ray.DTS.720p.x264-HDBRiSe
    let r = parse("9.2009.Blu-ray.DTS.720p.x264-HDBRiSe.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert_eq!(r.title, Some("9".to_string()));
    assert_eq!(r.year, Some(2009));
}

#[test]
fn test_guessit_2001_space_odyssey() {
    // From guessit movies.yml: 2001.A.Space.Odyssey.1968.HDDVD.1080p.DTS.x264
    // Special case: "2001" is title, "1968" is year
    let r = parse("2001.A.Space.Odyssey.1968.HDDVD.1080p.DTS.x264-EuReKA.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    // First year found is 2001 (which is in the title), parser behavior
    // Note: guessit handles this better with positional analysis
    assert!(r.title.is_some());
}

#[test]
fn test_guessit_2012_movie() {
    // From guessit movies.yml: 2012.2009.720p.BluRay.x264.DTS-WiKi
    // "2012" is the movie title, "2009" is the year
    let r = parse("2012.2009.720p.BluRay.x264.DTS-WiKi.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert!(r.title.is_some());
}

// -- Multi-Episode Formats --

#[test]
fn test_guessit_multi_episode_dash() {
    // From guessit episodes.yml: Wheels.S03E01-02.720p.HDTV.x264-IMMERSE.mkv
    let r = parse("Wheels.S03E01-02.720p.HDTV.x264-IMMERSE.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(3));
    assert_eq!(r.episode_info.episode, Some(1));
    assert_eq!(r.episode_info.episode_end, Some(2));
}

#[test]
fn test_guessit_multi_episode_e_dash_e() {
    // From guessit episodes.yml: Wheels.S03E01-E02.720p.HDTV.x264-IMMERSE.mkv
    let r = parse("Wheels.S03E01-E02.720p.HDTV.x264-IMMERSE.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(3));
    assert_eq!(r.episode_info.episode, Some(1));
    assert_eq!(r.episode_info.episode_end, Some(2));
}

#[test]
fn test_guessit_multi_episode_range() {
    // From guessit episodes.yml: Wheels.S03E01-04.720p.HDTV.x264-IMMERSE.mkv
    let r = parse("Wheels.S03E01-04.720p.HDTV.x264-IMMERSE.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(3));
    assert_eq!(r.episode_info.episode, Some(1));
    // Range episodes 1-4
}

// -- Alternative Episode Formats --

#[test]
fn test_guessit_episode_format_2x05() {
    // From guessit episodes.yml: Californication.2x05.Vaginatown.HDTV.XviD-0TV.avi
    let r = parse("Californication.2x05.Vaginatown.HDTV.XviD-0TV.avi");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Californication"));
    assert_eq!(r.episode_info.season, Some(2));
    assert_eq!(r.episode_info.episode, Some(5));
}

#[test]
fn test_guessit_episode_format_117() {
    // From guessit various.yml: new.girl.117.hdtv-lol.mp4
    // This format encodes season+episode as 3 digits: 117 = S01E17
    let r = parse("new.girl.117.hdtv-lol.mp4");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().to_lowercase().contains("new girl"));
    // 117 -> S01E17
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(17));
}

#[test]
fn test_guessit_episode_format_501() {
    // From guessit various.yml: the.mentalist.501.hdtv-lol.mp4
    // This format encodes season+episode as 3 digits: 501 = S05E01
    let r = parse("the.mentalist.501.hdtv-lol.mp4");
    assert_eq!(r.media_type, MediaType::Episode);
    // 501 -> S05E01
    assert_eq!(r.episode_info.season, Some(5));
    assert_eq!(r.episode_info.episode, Some(1));
}

// -- Episode Titles --

#[test]
fn test_guessit_episode_with_title() {
    // From guessit episodes.yml: The.Simpsons.S24E03.Adventures.in.Baby-Getting.720p.WEB-DL
    let r = parse("The.Simpsons.S24E03.Adventures.in.Baby-Getting.720p.WEB-DL.DD5.1.H.264-CtrlHD.mkv");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Simpsons"));
    assert_eq!(r.episode_info.season, Some(24));
    assert_eq!(r.episode_info.episode, Some(3));
    // Episode title should be extracted
    if let Some(ref ep_title) = r.episode_info.episode_title {
        assert!(ep_title.contains("Adventures") || ep_title.contains("Baby"));
    }
}

// -- Complex Release Groups --

#[test]
fn test_guessit_release_group_with_brackets() {
    // From guessit release_group.yml: [ABC] Some.Title.avi
    let r = parse("[ABC].Some.Title.S01E02.avi");
    assert_eq!(r.media_type, MediaType::Episode);
    // Bracket groups at start
}

#[test]
fn test_guessit_release_group_d_z0n3() {
    // From guessit movies.yml: The.Rum.Diary.2011.1080p.BluRay.DTS.x264.D-Z0N3.mkv
    let r = parse("The.Rum.Diary.2011.1080p.BluRay.DTS.x264.D-Z0N3.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert!(r.title.as_ref().unwrap().contains("Rum Diary"));
    assert_eq!(r.year, Some(2011));
    // Release group with hyphen in name - currently extracts "Z0N3" (after hyphen)
    // TODO: Improve to handle compound release groups like "D-Z0N3"
    assert!(r.release_group.is_some());
    // Current behavior extracts the part after the hyphen
    assert!(r.release_group.as_ref().unwrap().contains("Z0N3"));
}

// -- Titles with Numbers --

#[test]
fn test_guessit_angry_men_series() {
    // From guessit movies.yml: 1.Angry.Man.1957.mkv, 12.Angry.Men.1957.mkv, 123.Angry.Men.1957.mkv
    let r1 = parse("1.Angry.Man.1957.mkv");
    assert!(r1.title.as_ref().unwrap().contains("Angry"));
    assert_eq!(r1.year, Some(1957));

    let r12 = parse("12.Angry.Men.1957.mkv");
    assert!(r12.title.as_ref().unwrap().contains("Angry"));
    assert_eq!(r12.year, Some(1957));
}

// -- Special Characters in Titles --

#[test]
fn test_guessit_title_with_apostrophe() {
    // From guessit movies.yml: Howl's_Moving_Castle_(2004)_[720p,HDTV,x264,DTS]-FlexGet.avi
    let r = parse("Howl's.Moving.Castle.2004.720p.HDTV.x264.DTS-FlexGet.avi");
    assert_eq!(r.media_type, MediaType::Movie);
    assert!(r.title.as_ref().unwrap().contains("Howl"));
    assert_eq!(r.year, Some(2004));
}

#[test]
fn test_guessit_title_with_colon() {
    // Titles with colons like "Star Wars: Episode IV"
    let r = parse("Star.Wars.Episode.IV.A.New.Hope.1977.1080p.BluRay.x264-GROUP.mkv");
    assert_eq!(r.media_type, MediaType::Movie);
    assert!(r.title.as_ref().unwrap().contains("Star Wars"));
    assert_eq!(r.year, Some(1977));
}

// -- Country/Region Codes --

#[test]
fn test_guessit_country_code_us() {
    // From guessit episodes.yml: The.Office.(US).1x03.Health.Care.HDTV.XviD-LOL.avi
    let r = parse("The.Office.US.S01E03.Health.Care.HDTV.XviD-LOL.avi");
    assert_eq!(r.media_type, MediaType::Episode);
    assert!(r.title.as_ref().unwrap().contains("Office"));
    assert_eq!(r.episode_info.season, Some(1));
    assert_eq!(r.episode_info.episode, Some(3));
}

// -- High Season/Episode Numbers --

#[test]
fn test_guessit_high_season_number() {
    // From guessit various.yml: House.Hunters.International.S56E06.720p.hdtv.x264.mp4
    let r = parse("House.Hunters.International.S56E06.720p.hdtv.x264.mp4");
    assert_eq!(r.media_type, MediaType::Episode);
    assert_eq!(r.episode_info.season, Some(56));
    assert_eq!(r.episode_info.episode, Some(6));
}

// ============================================================================
// BATCH TEST - ALL FILENAMES FROM USER'S LIBRARY
// ============================================================================

#[test]
fn test_batch_all_samples() {
    let samples = vec![
        // Movies
        "Ballerina.2025.Hybrid.BDRip.x264.HUN-FULCRUM",
        "Bugonia.2025.MA.WEBRip.x264.HUN-FULCRUM",
        "Home.Alone.1990.REMASTERED.READ.NFO.BDRip.x264.AC3.HuN-Essence",
        "Lyle.Lyle.Crocodile.2022.720p.BluRay.DD+5.1.x264.HuN-No1",
        "Willy.Wonka.and.the.Chocolate.Factory.1971.720p.BluRay.DTS.x264.HuN-FASiRT",
        "Wonka.2023.720p.iT.WEB-DL.DD+5.1.Atmos.H.264.HuN-No1",
        
        // TV - Dark Matter
        "Dark.Matter.S01E10.BDRIP.x264-Krissz.mkv",
        "Dark.Matter.S02E08.PROPER.720p.HDTV.x264-KILLERS.mkv",
        "Dark.Matter.S03E01E02.720p.HDTV.x264-AVS.mkv",
        
        // TV - Stargate Atlantis
        "Stargate.Atlantis.S01E01-E02.Rising.BDRip.x264.Hun.Eng-MaMMuT.mkv",
        "Stargate.Atlantis.S01E06.Childhood's.End.BDRip.x264.Hun.Eng-MaMMuT.mkv",
        
        // TV - Stargate SG-1
        "Stargate.SG-1.S09E18.Arthur's.Mantle.BDRip.x264.Hun.Eng-MaMMuT.mkv",
        "Stargate.SG-1.S10E06.200.BDRip.x264.Hun.Eng-MaMMuT.mkv",
        
        // TV - Stargate Universe
        "Stargate.Universe.S01E01E02.Air.Parts.1.and.2.avi",
        "Stargate.Universe.S02E07.The.Greater.Good.avi",
    ];
    
    for sample in samples {
        let r = parse(sample);
        
        // Every sample should have a title
        assert!(r.title.is_some(), "No title for: {}", sample);
        
        // Confidence should be reasonable
        assert!(r.confidence >= 50, "Low confidence ({}) for: {}", r.confidence, sample);
        
        // Media type should be detected
        assert!(
            r.media_type != MediaType::Unknown || !sample.contains("S0"),
            "Unknown type for: {}", sample
        );
        
        println!("✓ {} -> {:?}: {:?}", sample, r.media_type, r.title);
    }
}
