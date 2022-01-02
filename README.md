# clicklauncher
clicklauncher is a small tool that runs different application programs based on the number of mouse clicks or key types.

It was developed to be used with window managers such as sway.
## Usage
1. Prepare a tab-delimited file with the sequence of identification numbers given in the arguments and the command to be executed.
2. Run clicklauncher with the number given as an argument for identification.
### Config file location
- The location of the configuration file can be specified with the -c option.
- The default location for the configuration file is $XDG_CONFIG_HOME/clicklauncher/cmdtable.tsv on Linux.
- If you want to know the default location, please run the command with --help option.
## Help message
```
$ clicklauncher --help
clicklauncher 0.3.0
nilninull <nilninull@gmail.com>
Launcher that switches programs according to the number of executions

USAGE:
    clicklauncher [OPTIONS] <ID>

ARGS:
    <ID>    click id number

OPTIONS:
    -c, --config <FILE>...    Sets a custom config file [default:
                              ~/.config/clicklauncher/cmdtable.tsv]
    -h, --help                Print help information
    -s, --msecs <MSECS>...    click separation time by milli seconds [default: 250]
    -V, --version             Print version information
```
## Example
### Window manager settings (from my sway config)
```
bindsym --whole-window BTN_FORWARD exec clicklauncher 1
bindsym --whole-window BTN_BACK exec clicklauncher 2
```
### Config file
```
# Lines starting with # are comments
# This file is tab separated table
# 1st column: space separated id number sequence
# 2nd column: command line string until the end of line
1       notify-send clicklauncher 'single click!'
1 1     notify-send clicklauncher 'double click!!'
1 1 1   notify-send clicklauncher 'triple click!!!'
2       copyq menu
2 2     notify-send "$(date +'%a %d  %T')" "$(cal)"
2 2 2   notify-send clicklauncher "$(cat ~/.config/clicklauncher/cmdtable.tsv)"
1 2     notify-send clicklauncher 'compound ids 1 -> 2'
2 1     notify-send clicklauncher 'compound ids 2 -> 1'
```
## Tips
### Wayland
The event names for keystrokes and mouse clicks can be found in libinput application.
```
$ libinput debug-events
```
### X window manager
The event names for keystrokes and mouse clicks can be found in xev application.
```
$ xev
```
