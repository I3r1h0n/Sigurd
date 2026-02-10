<div>
<img height="250" align="left" style="float: left; margin: 0 20px 0 0px;" alt="Sigurd logo" src="assets/sigurd-2.jpg">

<h1>Sigurd</h1>

<p>Sigurd <i>(Old Norse: Sigurðr)</i> was a legendary Norse hero who killed the dragon Fafnir and possessed the cursed treasure. <p>

<div align="center">
<img alt="Rust" src="https://img.shields.io/badge/rust-f04041?style=for-the-badge&labelColor=c0282d&logo=rust">
<img alt="Version" src="https://img.shields.io/badge/version-v0.1.0-green?style=for-the-badge">
<img alt="Lic" src="https://img.shields.io/github/license/I3r1h0n/Sigurd?label=license&style=for-the-badge">
</div>
</div>

<div style="clear: both;"></div>

## Overview
`Sigurd` is a BYOVD (aka Bring Your Own Vulnerable Driver) exploitation tool, made to kill processes. This tool allow you to prepare custom config(toml or json), or config it on run using TUI, making it easy to use. It also allow you to exploit multiply drivers, without carrying them around (you choose what to include at compile time). 

> [!WARNING]
> This tool was created for authorized security research and testing only. The authors and distributors accept no liability for misuse. Before using it, make sure you have lawful authorization and know what are you doing. Happy pwning!

### Table of content

- [Details](#details)
- [Building guide](#build)
- [Working demo](#demo)
- [Drivers](#drivers)
    - [Implemented drivers](#implemented-drivers)
    - [Details on Throttle Stop](#throttle-stop-details)
    - [References](#references)
- [Contribution guide](#contribution)
- [Creds](#creds)

## Details
BYOVD technique implies installing a vulnerable and signed driver on system, in order to exploit it's known vulnerability to gain privileges, read system secrets or (in our case) - kill processes. You can read more about it at [Microsoft Security Experts Blog](https://techcommunity.microsoft.com/blog/microsoftsecurityexperts/strategies-to-monitor-and-prevent-vulnerable-driver-attacks/4103985).

To find some details and links to articles about used drivers, see [Drivers](#drivers) section.

## Build

All you need is lastest rust tool chain on you Windows machine (_or on any other machine, in case you know what are you doing_). You can find the standalone installers [here](https://forge.rust-lang.org/infra/other-installation-methods.html#standalone-installers). 

After installing rust toolchain, just clone the repository and enter project directory:
```shell
git clone https://github.com/I3r1h0n/Sigurd
cd Sigurd/sigurd
```

Now it all depends on what drivers you want to use. You can include only desired ones, by changing the set of cargo features. Below is an example build command, with basic set of drivers and no trace messages:
```shell
cargo build --release --no-default-features --features "throttlestop bdapiutil64 k7rkscan wsftprm"
```

After build is finished, you can find binary in `/sigurd/target/release` folder.

## Demo

Usage is pretty simple. Below is the help output. 
```shell
> .\sigurd.exe --help
BYOVD technique

Usage: sigurd.exe [OPTIONS]

Options:
  -c, --config <CONFIG>                Path to .toml config file
      --config-string <CONFIG_STRING>  TOML configuration as a quoted string
  -s, --silent                         Run app without interface
  -h, --help                           Print help
  -V, --version                        Print version
```

Default config may look like this:
```toml
driver_name = "ThrottleStop"
installation_path = 'C:\ProgramData'
victim_processes = [
    "notepad.exe",
]
continuous = false
uninstall = true
```

You can:
1. Save it next to executable as `Config.toml`, 
2. Save it somewhere else, and provide it's path via `--config`
3. Convert it to valid JSON and pass it as `--config-string`
4. Or start without any config and configure Sigurd on run 

Silent mode allows you to run without starting a Terminal User Interface. Just provide a valid config, and sigurd will use it as is.

Here is the demo showing it use the ThrottleStop.sys to kill notepad.exe and MsMpEng.exe:
<div align="center"><img src="assets/demo.png"></div><br>

## Drivers

### Implemented drivers

Table of the currently implemented drivers.

|Driver|Version|CVE|Details|Status|
|------|-------|--------|-------|------|
|GameDriverX64|7.23.4.7|[CVE-2025-61155](https://nvd.nist.gov/vuln/detail/CVE-2025-61155)|[Blog](https://vespalec.com/blog/tower-of-flaws/)|Non on LoL|
|K7 driver|15.1.0.6|[CVE-2025-1055](https://nvd.nist.gov/vuln/detail/CVE-2025-1055)|[LolDrivers](https://www.loldrivers.io/drivers/9f88300d-e607-4e50-8626-fd799439e049/)|On LoL|
|ThrottleStop|3.0.0.0|[CVE-2025-7771](https://nvd.nist.gov/vuln/detail/CVE-2025-7771)|[SecureList](https://securelist.com/av-killer-exploiting-throttlestop-sys/117026/)|Not on LoL|
|BdApiUtil64|5.0.3.18797|[CVE-2024-51324](https://nvd.nist.gov/vuln/detail/CVE-2024-51324)|[LolDrivers](https://github.com/magicsword-io/LOLDrivers/issues/204)|On LoL|
|WSFTPrm|2.0.0.0|[CVE-2023-52271](https://nvd.nist.gov/vuln/detail/CVE-2023-52271)|[research](https://northwave-cybersecurity.com/vulnerability-notice-topaz-antifraud)|Not on LoL?|
|wamsdk|1.1.100|-|[Checkpoint](https://research.checkpoint.com/2025/silver-fox-apt-vulnerable-drivers/)|Blocked|
|KsAPI64|1.0.591.131|-|-|Blocked|

<br>

You can find all driver files in `sigurd/drivers` folder.

I also didn't include the `ksapi64` and `wamsdk` driver to default features list, because it's been blocked by windows vulnerable driver block list.

### ThrottleStop details
ThrottleStop is a special case, since it's not so 'naive' BYOVD EDR Killer driver. It allow an arbitrary physical memory read/write, and because of that - exploiting it as a EDR killer is a little more complicated then just sending a correct struct in IOCTL request. See the [details](/details/ThrottleStop.md)

### References

Creation of Sigurd is higly inspired by [this](https://github.com/BlackSnufkin/BYOVD) project by [BlackSnufkin](https://github.com/BlackSnufkin). Also big thanks to Kaspersky for thair analyze on ThrottleStop.

## Contribution

If you have an idea on how to improve this project, want to report a bug, or willing to implement another driver exploit - feel free to open an issue or pull request. 

All you need to add a new driver to sigurd is implement a `KillerDriver` trait. See it in the `/sigurd/src/drivers/mod.rs` and check the `/sigurd/src/drivers/k7rkscan/mod.rs` as an example.

## Creds

prod by _I3r1h0n_.
