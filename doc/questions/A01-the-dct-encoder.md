Use mozjpeg-rs 

However be very careful to only reencode when explicitly requested.
Unless a specific format is requested, we should always extract the binary data into the corresponding image format (*without reencoding*)

Use any pure image encoding libraries for other formats (if the license allows it).

