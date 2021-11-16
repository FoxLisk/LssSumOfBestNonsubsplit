# LssSumOfBestNonsubsplit
Simple Rust project to calculate sum of best segments for a LiveSplit file that has subsplits.

LiveSplit has a built in Sum of Best Segments calculation, however for splits that contain subsplits this adds up the best segments from each individual subsplit.  This makes it difficult to compare Sum of Best to runners who use segments without subsplits. 

This application takes a LiveSplit file containing subsplits and outputs both the current LiveSplit Sum of Best Segments and the Sum of Best Non-Subsplits for each segment and the total.

To use, run the executable from a command line and provide the path to the target livesplit file as an argument, for example:
> sum_of_best_nonubsplit_segments.exe C:/path/to/MySplitFile.lss

The project has been tested on Windows only, though being done in Rust it *should* work on Linux, but no guarantees.
