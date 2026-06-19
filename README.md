# Litnergy
[English](README.en.md)
[简体中文](README.md)

一个主要用于Deskflow的命令行客户端, 主要解决deskflow客户端模式在部分linux wayland协议的de或wm下不一定正常运作的问题。 ~~(但其实我也没试过deskflow客户端, 只是deskflow的known issue写了不支持)~~

解决方案是利用linux `uinput` 模块, 让共享键盘和鼠标输入直接发送到内核, 然后被内核和用户空间消费。该模块只与`uinput`模块有关，因此理论上在任何编译了该模块的发行版上都可以运行。

此客户端在niri上可以正常运行（即我本人的环境）

## 使用方式
确保你的系统开启了`uinput`模块。

```bash
modprobe uinput 
```

确保Deskflow服务端正常运行。litnergy运行参数必须提供服务端地址和客户端屏幕分辨率。

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

出现`uinput`相关模块问题请自行搜索，这与发行版环境有关。

## Requirement
<!-- You should make sure the following shared libs exist. -->
确保你的系统安装了以下库:
- `libevdev`

## 更新日志

### v1.1.0
实现剪切板同步文本和bmp格式的图片

可能不支持文本图片混合复制

### v1.0.0
基本的键鼠共享 

未对小键盘等特殊处理, 兼容不佳

## 计划
- [x] 鼠标共享
- [x] 键盘共享
- [x] 剪切板同步
- [ ] 支持MacOS
- [ ] 对鼠标侧建和更多键盘键位支持

