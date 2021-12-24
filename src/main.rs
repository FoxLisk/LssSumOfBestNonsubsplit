use std::{
    collections::BTreeMap,
    fmt,
    fs::File,
    io::{self, prelude::*},
    str,
};

use time::{Date, Duration, PrimitiveDateTime, Time};
use xmltree::Element;

use serde::Deserialize;

extern crate clap;
use clap::App;
use std::path::Path;
use std::fs::read_to_string;

#[derive(Deserialize)]
struct SubSegment {
    name: String,
    start: String,
    #[serde(default)]
    end: Option<String>,
}

struct SubSegmentContext {
    sub_segment: SubSegment,
    length: Time,
    length_non_subsplit: Time,
    active: bool,
}

impl SubSegmentContext {
    fn new(sub_segment: SubSegment) -> Self {
        Self {
            sub_segment,
            length: Time::try_from_hms_nano(0, 0, 0, 0).unwrap(),
            length_non_subsplit: Time::try_from_hms_nano(0, 0, 0, 0).unwrap(),
            active: false,
        }
    }

    fn slurp(&mut self, segment: &Segment) {
        if self.sub_segment.start == segment.name {
            self.active = true;
        }
        if self.active {
            self.length += time_to_duration(&segment.sum_of_best);
            self.length_non_subsplit += time_to_duration(&segment.sum_of_best_nonsubsplit);
        }

        match &self.sub_segment.end {
            Some(e) => {
                if e == &segment.name {
                    self.active = false;
                }
            }
            None => {}
        }
    }
}

fn main() {
    let matches = App::new("Sum of Best Tool")
        .version("1.3")
        .about("Calculates sum of best for a range of segments and subsplits, outputs subsplit and non-subsplit totals")
        .args_from_usage(
            "<LSS_FILE>             'Filename (including path) of the LSS File to parse'
            -s, --start_segment=[SEGMENT_NAME]  'Name of the segment to start calculating from (inclusive)'
            -e, --end_segment=[SEGMENT_NAME]  'Name of the segment to end calculating from (inclusive)'
            -c, --config=[CONFIG_FILE] 'A JSON config file of subsegments to calculate'
            ")
        .get_matches();

    let filename = matches.value_of("LSS_FILE").unwrap();
    let mut sub_segments = Vec::new();
    if let Some(start_segment) = matches.value_of("start_segment") {
        sub_segments.push(SubSegmentContext::new(SubSegment {
            name: "Custom".to_owned(),
            start: start_segment.to_owned(),
            end: matches.value_of("end_segment").map(|s| s.to_owned()),
        }));
    }

    if let Some(sub_segment_config) = matches.value_of("config") {
        let p = Path::new(sub_segment_config);
        if !p.exists() {
            println!("Error: could not find config file {}", sub_segment_config);
        } else {
            let c = read_to_string(p).unwrap();
            let deserialized: Vec<SubSegment> = serde_json::from_str(&*c).unwrap();
            for s in deserialized {
                sub_segments.push(
                  SubSegmentContext::new(s)
                );
            }
        }
    }

    let root_element = parse_lss_file(filename.to_string());

    // We don't currently use the attempt history, but leaving it in case there's a desire to print info about which attempt(s)
    // the best segments came from
    let _attempt_history = build_attempt_history(&root_element);
    // #[cfg(debug_assertions)]
    // {
    //     for (attempt_id, attempt_date_time) in &attempt_history
    //     {
    //         println!("Attempt {} started on {}", attempt_id, attempt_date_time);
    //     }
    // }

    let segments = build_segments(&root_element);

    if let Some(s) = segments.first() {
        sub_segments.push(SubSegmentContext::new(SubSegment {
            name: "All Splits".to_owned(),
            start: s.name.clone(),
            end: None,
        }));
    }

    for segment in &segments {
        for sub_segment in &mut sub_segments {
            sub_segment.slurp(segment);
        }
        println!("{}", segment.name);
        println!("  Best Segment: {}", segment.sum_of_best);
        if segment.has_subsplits {
            println!(
                "  Best Segment NonSubsplit: {}",
                segment.sum_of_best_nonsubsplit
            );
        }
    }
    println!();

    for sub_segment in sub_segments {
        println!("{}", sub_segment.sub_segment.name);
        println!("  Best Segment: {}", sub_segment.length);
        if sub_segment.length_non_subsplit > Time::try_from_hms_nano(0, 0, 0, 1).unwrap() {
            println!(
                "  Best Segment NonSubsplit: {}",
                sub_segment.length_non_subsplit
            );
        }
    }
}

fn time_to_duration(time: &Time) -> Duration {
    let total_seconds = i64::from(time.second())
        + (i64::from(time.minute()) * 60)
        + (i64::from(time.hour()) * (60 * 60));
    return Duration::new(total_seconds, time.nanosecond() as i32);
}

// Opens a LSS file and parses it as XML.
fn parse_lss_file(filename: String) -> Element {
    let buf = {
        let r = File::open(filename).unwrap();
        let mut reader = io::BufReader::new(r);

        // xmltree does not properly handle the prolog line, so we have to strip it out when reading the file
        reader.read_until(b'\n', &mut Vec::new()).unwrap();

        // Read the rest of the file into a buffer to be parsed
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        buf
    };
    // Convert the file contents to a str, then parse
    let contents = str::from_utf8(&buf).unwrap();

    Element::parse(contents.as_bytes()).unwrap()
}

// Parses the LSS <AttemptHistory> node, converting the <Attempt> nodes to a vector of PrimitiveDateTime objects
fn build_attempt_history(root: &xmltree::Element) -> BTreeMap<u32, PrimitiveDateTime> {
    let mut attempt_history: BTreeMap<u32, PrimitiveDateTime> = BTreeMap::new();

    // Pull out the attempt history then iterate through the attempts
    let attempt_history_root = root
        .get_child("AttemptHistory")
        .expect("Can't find 'AttemptHistory' node");
    for child in &attempt_history_root.children {
        if let xmltree::XMLNode::Element(child_element) = child {
            // We only expect Attempt Elements
            if child_element.name == "Attempt" {
                // Pull out the started time string and convert it to a PrimitiveDateTime to add to our vector
                let attempt_id_str = child_element
                    .attributes
                    .get("id")
                    .expect("Attempt is missing ID");
                let attempt_id = attempt_id_str.parse::<u32>().unwrap();
                let start_time = child_element
                    .attributes
                    .get("started")
                    .expect("Attempt is missing started time");
                let date_time_vec = start_time.split(" ").collect::<Vec<_>>();

                let date_parts = date_time_vec[0].split("/").collect::<Vec<_>>();
                let month = date_parts[0].parse::<u8>().unwrap();
                let day = date_parts[1].parse::<u8>().unwrap();
                let year = date_parts[2].parse::<i32>().unwrap();
                let date = Date::try_from_ymd(year, month, day).unwrap();

                let time_parts = date_time_vec[1].split(":").collect::<Vec<_>>();
                let hours = time_parts[0].parse::<u8>().unwrap();
                let minutes = time_parts[1].parse::<u8>().unwrap();
                let seconds = time_parts[2].parse::<u8>().unwrap();
                let time = Time::try_from_hms(hours, minutes, seconds).unwrap();

                let date_time = PrimitiveDateTime::new(date, time);
                attempt_history.insert(attempt_id, date_time);
            }
        }
    }

    return attempt_history;
}

fn calc_total_seconds(hours: u8, minutes: u8, seconds: u8) -> i64 {
    return i64::from(seconds) + (i64::from(minutes) * 60) + (i64::from(hours) * (60 * 60));
}

// Utility function that converts a LSS <RealTime> node string into a Duration structure
fn build_duration_from_realtime_str(realtime: &str) -> Duration {
    let time_parts = realtime.split(":").collect::<Vec<_>>();
    let hours = time_parts[0].parse::<u8>().unwrap();
    let minutes = time_parts[1].parse::<u8>().unwrap();
    let sec_ms_parts = time_parts[2].split(".").collect::<Vec<_>>();
    let seconds = sec_ms_parts[0].parse::<u8>().unwrap();
    let mut nano = 0;
    if sec_ms_parts.len() > 1 {
        // <RealTime> nodes have 7 decimal places, need to add a couple trailing zeroes to convert to nanoseconds
        nano = sec_ms_parts[1].parse::<i32>().unwrap() * 100;
    }

    return Duration::new(calc_total_seconds(hours, minutes, seconds), nano);
}

#[derive(Clone)]
struct SubSplit {
    name: String,
    attempts: BTreeMap<u32, Duration>,
    best_time: Duration,
}

impl fmt::Display for SubSplit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubSplit: {}, best time: {:?}",
            self.name, self.best_time
        )
        .expect("Write for subsplit display failed unexpectedly");
        for (attempt_id, attempt_time) in &self.attempts {
            write!(
                f,
                "\n    Attempt ID {} completed in {:?}",
                attempt_id, attempt_time
            )
            .expect("Write for subsplit display failed unexpectedly");
        }
        write!(f, "\n")
    }
}

// Parses a single <Segment> node into a SubSplit structure
fn build_subsplit(subsplit_root: &xmltree::Element) -> SubSplit {
    let subsplit_name_str = &subsplit_root.get_child("Name").unwrap().get_text().unwrap();
    let best_time_root = &subsplit_root
        .get_child("BestSegmentTime")
        .unwrap()
        .get_child("RealTime")
        .unwrap();

    let mut subsplit = SubSplit {
        name: subsplit_name_str.to_string(),
        attempts: BTreeMap::new(),
        best_time: build_duration_from_realtime_str(&best_time_root.get_text().unwrap()),
    };

    // #[cfg(debug_assertions)]
    // println!("Found SubSplit named {}", subsplit.name);

    let segment_history_root = &subsplit_root.get_child("SegmentHistory").unwrap();
    for child in &segment_history_root.children {
        if let xmltree::XMLNode::Element(child_time) = child {
            let time_id_str = child_time.attributes.get("id").expect("Time is missing ID");

            // For some reason, there are some negative time IDs. Just ignore those
            if let Ok(time_id) = time_id_str.parse::<u32>() {
                // Similarly, ignore nodes that have no <RealTime> child node
                if let Some(child_realtime) = child_time.get_child("RealTime") {
                    let realtime =
                        build_duration_from_realtime_str(&child_realtime.get_text().unwrap());

                    subsplit.attempts.insert(time_id, realtime);

                    // #[cfg(debug_assertions)]
                    // println!("  Subsplit attempt {} completed in {}", time_id, realtime);
                }
            }
        }
    }

    return subsplit;
}

#[derive(Debug)]
struct Segment {
    name: String,
    sum_of_best: Time,
    sum_of_best_nonsubsplit: Time,
    has_subsplits: bool,
}

// Parses the LSS <Segments> node, converting the <Segment> nodes into SubSplit and Segment objects
fn build_segments(root: &xmltree::Element) -> Vec<Segment> {
    let mut segment_list: Vec<Segment> = Vec::new();
    let segments_root = root
        .get_child("Segments")
        .expect("Can't find 'Segments' node");

    let num_segments = segments_root.children.len();
    let mut subsplit_list: Vec<SubSplit> = Vec::new();
    let mut has_subsplits = false;
    for i in 0..num_segments {
        if let xmltree::XMLNode::Element(child_segment) = &segments_root.children.get(i).unwrap() {
            if child_segment.name == "Segment" {
                let subsplit = build_subsplit(child_segment);

                // #[cfg(debug_assertions)]
                // println!("{}", subsplit);

                let is_segment = subsplit.name.chars().next().unwrap() != '-';
                if is_segment == false {
                    has_subsplits = true;
                }
                subsplit_list.push(subsplit);

                if is_segment {
                    let segment_subsplit = &subsplit_list.last().unwrap();
                    let sum_of_best_time = calc_sum_of_best(&subsplit_list);
                    let sum_of_best_nonsubsplit = calc_sum_of_best_nonsubsplit(&subsplit_list);

                    let segment = Segment {
                        name: segment_subsplit.name.clone(),
                        sum_of_best: sum_of_best_time,
                        sum_of_best_nonsubsplit: sum_of_best_nonsubsplit,
                        has_subsplits: has_subsplits,
                    };

                    segment_list.push(segment);
                    subsplit_list.clear();
                    has_subsplits = false;
                }
            }
        }
    }

    return segment_list;
}

fn calc_sum_of_best(subsplit_list: &Vec<SubSplit>) -> Time {
    let mut sum_of_best_time = Time::try_from_hms_nano(0, 0, 0, 0).unwrap();

    for subsplit in subsplit_list {
        sum_of_best_time += subsplit.best_time;
    }

    return sum_of_best_time;
}

fn calc_sum_of_best_nonsubsplit(subsplit_list: &Vec<SubSplit>) -> Time {
    let attempt_ids = subsplit_list[0].attempts.keys();
    // Initialize the current best attempt time to 23 hours.  If you have an actual speedrun with a single
    // segment containing attempts longer than 23 hours you're on your own.
    let mut curr_best = Time::try_from_hms_nano(23, 0, 0, 0).unwrap();

    for id in attempt_ids {
        let mut attempt_time = Time::try_from_hms_nano(0, 0, 0, 0).unwrap();
        let mut is_valid = true;
        for subsplit in subsplit_list {
            match subsplit.attempts.get(id) {
                Some(subsplit_time) => attempt_time += subsplit_time.clone(),
                None => {
                    is_valid = false;
                    break;
                }
            }
        }

        if is_valid {
            if attempt_time < curr_best {
                curr_best = attempt_time;
            }
        } else {
            continue;
        }
    }

    return curr_best;
}
