use std::{
    env,
    fs::File,
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
    println!("{:?}", args);

    if args.len() < 2
    {
    	panic!("ERROR: Specify the .lss file to parse as a command line argument.\n    Example: {} MySplits.lss", args[0]);
    }

    let filename = &args[1];

    let root_element = parse_file(filename.to_string());

    let attempt_history = build_attempt_history(&root_element);

	// for (i, attempt_date_time) in attempt_history.iter().enumerate()
	// {
	// 	println!("Attempt {} started on {}", (i+1), attempt_date_time);
	// }

	

}

fn parse_file(filename: String) -> Element
{
	let buf = {
        let r = File::open(filename).unwrap();
        let mut reader = io::BufReader::new(r);
        reader.read_until(b'\n', &mut Vec::new()).unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        buf
    };
    let contents = 	str::from_utf8(&buf).unwrap();

    Element::parse(contents.as_bytes()).unwrap()
}

struct SubSplit
{
	name: String,
	attempts: HashMap<u32, Time>,
	best_time: Time,
}

struct Segment
{
	name: String,
	children: Vec<SubSplit>,
	sum_of_best: Time,
	sum_of_best_nonsubsplit: Time,
}

fn build_attempt_history(root: &xmltree::Element) -> Vec<PrimitiveDateTime>
{
	let mut attempt_history: Vec<PrimitiveDateTime> = Vec::new();
	let attempt_history_root = root.get_child("AttemptHistory").expect("Can't find AttemptHistory root");
	for child in &attempt_history_root.children
	{
		if let xmltree::XMLNode::Element(child_element) = child
		{
			//println!("{:?}", child_element);
			let attempt_id = child_element.attributes.get("id").expect("Attempt is missing ID");
			let start_time = child_element.attributes.get("started").expect("Attempt is missing started time");
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
			attempt_history.push(date_time);
		}
	}

	return attempt_history;
}

//fn buildSegments(root: Element) -> Vec<Segment>
//{

//}


