# Force UTF-8 Tool for Windows Executables (forceu8exe)

This tool embeds a manifest that force I/O using UTF-8 on Widows executables.

They usually performe I/O using legacy encoding (e.g. ISO-8859-1 / Shift-JIS / GBK) by default.
To force Unicode on them, we have to use UTF-16 (wide characters) API instead.
However, other OSes allow us to use UTF-8 and Unicode in narrow strings.
This tool give Windows executables compatibility with other OSes.

# Prerequirements

- Windows SDK in Visual C++ (mt.exe)
- Cargo (the package manager of Rust)

# How to use

Add Cargo and `mt.exe` in PATH in advance.  x64 Native Tools Command Prompt seems to be the most easily accessible.

```pwsh
cargo install --git https://github.com/tats-u/forceu8exe.git
forceu8exe apply [path of .exe file]
```

# License

MIT

