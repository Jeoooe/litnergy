# Litnergy
[English](README.en.md)
[简体中文](README.md)

This is a simple deskflow client intended for some linux users whose de or wm not supported by official Deskflow application. 

The backend of litnergy is the `uinput` module, which makes the client work normally in any linux distro with this module in theory.

Litnergy works well on niri.

## Usage
Before using litnergy, you should make sure `uinput` module is available on your os.

```bash
modprobe uinput 
```

Make sure your deskflow are running in server mode.

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

## Changelog

### v1.1.0
Implemented clipboard synchronization for text and BMP image formats.

May not support mixed text and image copying.

### v1.0.0
Basic keyboard and mouse sharing.

No special handling for keypad, etc.; compatibility is limited.

## Roadmap
- [x] Mouse sharing
- [x] Keyboard sharing
- [x] Clipboard synchronization
- [ ] Support for macOS
- [ ] Support for mouse side buttons and more keyboard keys
