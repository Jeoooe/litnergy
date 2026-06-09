# Litnergy

This is a simple deskflow client intended for some linux users whose de or wm not supported by official Deskflow application. 
The backend of litnergy is the `uinput` module, which makes the client work normally in any linux distro with this module in theory.

## Usage
Before starting litnergy, you should ensure your deskflow are running in server mode.
```
./litnergy -h
A Deskflow client based on uinput.

Usage: litnergy [OPTIONS] --server <SERVER> --resolution <RESOLUTION>

Options:
      --server <SERVER>            Ip addr, example: 127.0.0.1
      --port <PORT>                Default: 24800
  -r, --resolution <RESOLUTION>    Example 1920x1080
  -c, --client-name <CLIENT_NAME>  Client name, default: "litnergy"
  -h, --help                       Print help
  -V, --version                    Print version
```

## Requirement
You should make sure the following shared libs exist.
- `libevdev`
- `libgcc`
- `libc`

## TODO
- [x] Shared mouse
- [x] Shared keyboard
- [ ] Shared clipboard
- [ ] Extra buttons of mouse or keys on keyboard
