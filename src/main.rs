use std::{
    env,
    fmt,
    fs::File,
    collections::BTreeMap,
    io::{self, prelude::*},
    str,
};

use xmltree::Element;
use time::{
	macros::*,
	PrimitiveDateTime,
	Date,
	Time
};

fn main()
{
    let args: Vec<String> = env::args().collect();

    if args.len() < 2
    {
    	panic!("ERROR: Specify the .lss file to parse as a command line argument.\n    Example: {} MySplits.lss", args[0]);
    }

    let filename = &args[1];

    let root_element = parse_lss_file(filename.to_string());

    let attempt_history = build_attempt_history(&root_element);
	// #[cfg(debug_assertions)]
	// {
	// 	for (attempt_id, attempt_date_time) in &attempt_history
	// 	{
	// 		println!("Attempt {} started on {}", attempt_id, attempt_date_time);
	// 	}
	// }

	let segments = build_segments(&root_element);
	// #[cfg(debug_assertions)]
	// {
	// 	for segment in &segments
	// 	{
	// 		println!("Segment {}", segment.name);
	// 		for subsplit in &segment.subsplits
	// 		{
	// 			println!("  {}", subsplit);
	// 		}
	// 	}
	// }

	// TODO - Need to convert the Time objects to Duration to be able to add them up
	// let mut sum_of_best = Time::try_from_hms_nano(0,0,0,0).unwrap();
	// let mut sum_of_best_nonsubsplit = Time::try_from_hms_nano(0,0,0,0).unwrap();
	// for segment in segments
	// {
	// 	sum_of_best += segment.sum_of_best;
	// 	sum_of_best_nonsubsplit += segment.sum_of_best_nonsubsplit;
	// }

	// println!("LSS Sum of Best: {}\nSum of Best Non-SubSplits: {}", sum_of_best, sum_of_best_nonsubsplit);
}

// Opens a LSS file and parses it as XML.
fn parse_lss_file(filename: String) -> Element
{
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
    let contents = 	str::from_utf8(&buf).unwrap();

    Element::parse(contents.as_bytes()).unwrap()
}

// Parses the LSS <AttemptHistory> node, converting the <Attempt> nodes to a vector of PrimitiveDateTime objects
fn build_attempt_history(root: &xmltree::Element) -> BTreeMap<u32, PrimitiveDateTime>
{
	let mut attempt_history: BTreeMap<u32, PrimitiveDateTime> = BTreeMap::new();
	
	// Pull out the attempt history then iterate through the attempts
	let attempt_history_root = root.get_child("AttemptHistory").expect("Can't find 'AttemptHistory' node");
	for child in &attempt_history_root.children
	{
		if let xmltree::XMLNode::Element(child_element) = child
		{
			// We only expect Attempt Elements
			if child_element.name == "Attempt"
			{
				// Pull out the started time string and convert it to a PrimitiveDateTime to add to our vector
				let attempt_id_str = child_element.attributes.get("id").expect("Attempt is missing ID");
				let attempt_id     = attempt_id_str.parse::<u32>().unwrap();
				let start_time     = child_element.attributes.get("started").expect("Attempt is missing started time");
				let date_time_vec  = start_time.split(" ").collect::<Vec<_>>();

				let date_parts = date_time_vec[0].split("/").collect::<Vec<_>>();
				let month      = date_parts[0].parse::<u8>().unwrap();
				let day 	   = date_parts[1].parse::<u8>().unwrap();
				let year       = date_parts[2].parse::<i32>().unwrap();
				let date       = Date::try_from_ymd(year, month, day).unwrap();

				let time_parts = date_time_vec[1].split(":").collect::<Vec<_>>();
				let hours      = time_parts[0].parse::<u8>().unwrap();
				let minutes    = time_parts[1].parse::<u8>().unwrap();
				let seconds    = time_parts[2].parse::<u8>().unwrap();
				let time       = Time::try_from_hms(hours, minutes, seconds).unwrap();

				let date_time  = PrimitiveDateTime::new(date, time);
				attempt_history.insert(attempt_id, date_time);
			}
		}
	}

	return attempt_history;
}

// Utility function that converts a LSS <RealTime> node string into a Time structure
fn build_time_from_realtime_str(realtime: &str) -> Time
{
	let time_parts   = realtime.split(":").collect::<Vec<_>>();
	let hours        = time_parts[0].parse::<u8>().unwrap();
	let minutes      = time_parts[1].parse::<u8>().unwrap();
	let sec_ms_parts = time_parts[2].split(".").collect::<Vec<_>>();
	let seconds      = sec_ms_parts[0].parse::<u8>().unwrap();
	let mut nano     = 0;
	if sec_ms_parts.len() > 1
	{
		nano = sec_ms_parts[1].parse::<u32>().unwrap();
	}

	return Time::try_from_hms_nano(hours, minutes, seconds, nano).unwrap();
}

#[derive(Clone)]
struct SubSplit
{
	name: String,
	attempts: BTreeMap<u32, Time>,
	best_time: Time,
}

impl fmt::Display for SubSplit
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "SubSplit: {}, best time: {}", self.name, self.best_time);
		for (attempt_id, attempt_time) in &self.attempts
		{
			write!(f, "\n    Attempt ID {} completed in {}", attempt_id, attempt_time);
		}
		write!(f, "\n")
	}
}

// Parses a single <Segment> node into a SubSplit structure
fn build_subsplit(subsplit_root: &xmltree::Element) -> SubSplit
{
	let subsplit_name_str = &subsplit_root.get_child("Name").unwrap().get_text().unwrap();
	let best_time_root = &subsplit_root.get_child("BestSegmentTime").unwrap().get_child("RealTime").unwrap();

	let mut subsplit = SubSplit
	{
		name 	  : subsplit_name_str.to_string(),
		attempts  : BTreeMap::new(),
		best_time : build_time_from_realtime_str(&best_time_root.get_text().unwrap()),
	};

	// #[cfg(debug_assertions)]
	// println!("Found SubSplit named {}", subsplit.name);

	let segment_history_root = &subsplit_root.get_child("SegmentHistory").unwrap();
	for child in &segment_history_root.children
	{
		if let xmltree::XMLNode::Element(child_time) = child
		{
			let time_id_str = child_time.attributes.get("id").expect("Time is missing ID");
			
			// For some reason, there are some negative time IDs. Just ignore those
			if let Ok(time_id) = time_id_str.parse::<u32>()
			{
				// Similarly, ignore nodes that have no <RealTime> child node
				if let Some(child_realtime) = child_time.get_child("RealTime")
				{
					let realtime = build_time_from_realtime_str(&child_realtime.get_text().unwrap());

					subsplit.attempts.insert(time_id, realtime);

					// #[cfg(debug_assertions)]
					// println!("  Subsplit attempt {} completed in {}", time_id, realtime);
				}
			}
		}
	}

	return subsplit;
}

struct Segment
{
	name: String,
	subsplits: Vec<SubSplit>,
	sum_of_best: Time,
	sum_of_best_nonsubsplit: Time,
}

// Parses the LSS <Segments> node, converting the <Segment> nodes into SubSplit and Segment objects
fn build_segments(root: &xmltree::Element) -> Vec<Segment>
{
	let mut segment_list: Vec<Segment> = Vec::new();
	let mut segments_root = root.get_child("Segments").expect("Can't find 'Segments' node");

	let num_segments = segments_root.children.len();
	let mut subsplit_list: Vec<SubSplit> = Vec::new();
	for i in 0..num_segments
	{
		if let xmltree::XMLNode::Element(child_segment) = &segments_root.children.get(i).unwrap()
		{
			if child_segment.name == "Segment"
			{
				let mut subsplit = build_subsplit(child_segment);
				
				// #[cfg(debug_assertions)]
				// println!("{}", subsplit);
				
				let is_segment = (subsplit.name.chars().next().unwrap() != '-');
				
				subsplit_list.push(subsplit);
				 
				if is_segment
				{
					let segment_subsplit = &subsplit_list.last().unwrap();
					let segment = Segment
					{
						name: segment_subsplit.name.clone(),
						subsplits: subsplit_list.to_vec(),
						sum_of_best: Time::try_from_hms_nano(0,0,0,0).unwrap(),
						sum_of_best_nonsubsplit: Time::try_from_hms_nano(0,0,0,0).unwrap(),
					};

					segment_list.push(segment);
					subsplit_list.clear();
				}
			}
		}
	}

	return segment_list;
}

// Parses a set of LSS <Segment> nodes into a Segment object
// fn build_segment(segment_root: &xmltree::Element) -> Segment
// {
// 	let mut segment: Segment;

// 	// Segments are a collection of <Segment> nodes with a leading '-', ending with a <Segment>
// 	// without the leading '-'

// 	return segment;

// }

// Parses a LSS <Segment> into a SubSplit object
