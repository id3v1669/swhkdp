swhks(1) "github.com/id3v1669/swhkd" "General Commands Manual"

# NAME

swhks - Server for swhkd, used to run user level commands over IPC.

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

Created by Shinyzenith <aakashsensharma@gmail.com>, EdenQwQ <lsahlm1eden@gmail.com>, and Angelo Fallaria <ba.fallaria@gmail.com>.
Maintained by id3v1669 <id3v1669@gmail.com> as original repo looks unmaintained for now.
For more information about development, see <https://github.com/id3v1669/swhks>.

# SEE ALSO

	- *swhkd(1)*
	- *swhkd(5)*
	- *swhkd-keys(5)*
