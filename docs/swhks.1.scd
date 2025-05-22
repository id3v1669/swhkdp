swhks(1) "github.com/id3v1669/swhkdp" "General Commands Manual"

# NAME

swhks - Server for swhkdp, used to run user level commands over IPC.

# SYNOPSIS

*swhks*

# OPTIONS

*-h*, *--help*
	Print help message and quit.

*-V*, *--version*
	Print version information.

*-l*, *--log* <LOG_FILE_PATH>
	Set a log file path.
	If *XDG_DATA_HOME* exists then we use *swhks/swhks-current_time.log* relative to
	it, else we use *.local/share/swhks/swhks-current_time.log* relative to the
	user home directory.

*-d*, *--debug*
	Enable debug mode.

# AUTHORS

Created by Shinyzenith, EdenQwQ, and Angelo Fallaria.
Maintained by id3v1669 <id3v1669@gmail.com> as this project went different path from original repo.
For more information about development, see <https://github.com/id3v1669/swhks>.

# SEE ALSO

	- *swhkdp(1)*
	- *swhkdp(5)*
