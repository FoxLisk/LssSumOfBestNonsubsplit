use std::{
    env,
    fs::File,
    io::{self, prelude::*},
    str,
};

use xmltree::Element;

fn main()
{
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let filename = &args[1];

    let mut root_element = parseFile(filename.to_string());

	println!("{:#?}", root_element);
}

fn parseFile(filename: String) -> Element
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

// TODO - implement PartialOrd for comparison
struct RealTime
{
	// fn as_string()
	// equal and compare

	 hours:   u32;
	 minutes: u32;
	 seconds: float;
};

struct SubSplit
{
	name: str;
	// vector of ID/attempt pairs
	bestTime: RealTime;

	//fn getTimeForAttempt(attemptId: u32) -> RealTime
	//{

	//}

	//fn getBest() -> RealTime
	//{

	//}
};

struct AttemptHistory
{
	// TODO - Find time/date data format to use
	//attemptStartTimes: Vec<TimeDate>;

	//fn getStartTimeById(attemptId: u32) -> RealTime
	//{

	//}	
};

struct Segment
{
	name: str;
	children: Vec<SubSplit>;
	sumOfBest: RealTime;
	sumOfBestNonSubsplit: RealTime;

	//fn getSumOfBest(segment: Segment) -> RealTime
	//{

	//}

	//fn getSumOfBestNonSubsplit(segment: Segment) -> RealTime
	//{

	//}

};

//fn buildAttemptHistory(root: Element) -> AttemptHistory
//{

//}

//fn buildSegments(root: Element) -> Vec<Segment>
//{

//}


