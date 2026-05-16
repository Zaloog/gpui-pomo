# gpui-pomo

<p align="center"><img src="https://raw.githubusercontent.com/Zaloog/gpui-pomo/main/assets/app_icon.png" /></p>


A minimal pomodoro application using [gpui]

## Features
`gpui-pomo` comes with the following features:

- configurable session length with persistant config
- vim-like navigation
- jumps to foreground and gets focused when switching between `Focus` and `Break`
- utilizes [objc2-app-kit] to play sounds (`Blow.aiff` for `Breaks` and `Glass.aiff` for `Focus` sessions) from `/System/Library/Sounds/` when switching between `Focus` and `Break` sessions.

## Timer Screen
The `timer screen` is the main screen of the app.

Available shortcuts:

| key | description |
|- | -|
|q | quit the app |
|s | go to [Settings Screen](#settings-screen) |
|r | reset timer (and apply new settings, if setting changes are pending) |
|space | start or pause the timer |

<p align="center"><img src="https://raw.githubusercontent.com/Zaloog/gpui-pomo/main/assets/timer.png" /></p>

## Settings Screen
The `settings screen` can be used to configure the app. All settings persist under `~/.config/gpui-pomo/config.json`.
If Settings are pending there is a small red dot indicating that.

Available shortcuts:

| key | description |
|- | -|
|q | quit the app |
|s, escape | go back to [Timer Screen](#timer-screen) |
|k, j | move up/down |
|space, enter | go to [Settings Edit Screen](#settings-edit-screen) of the current setting|

<p align="center"><img src="https://raw.githubusercontent.com/Zaloog/gpui-pomo/main/assets/settings.png" /></p>

## Settings Edit Screen
The `settings edit screen` can be used to configure the values for the `Focus Time`, `Break Time` and `Total Sessions`.

Available shortcuts:

| key | description |
|- | -|
|q | quit the app |
|escape | go back to [Settings Screen](#settings-screen) |
|k, j | increase/decrease value |
|space, enter | confirm new value and go back to [Settings Screen](#settings-screen)|

<p align="center"><img src="https://raw.githubusercontent.com/Zaloog/gpui-pomo/main/assets/settings_edit.png" /></p>

## Installation
You can either clone the repo and run the app with 
```bash
cargo run

```

or use [cargo-bundler] to bundle it into an executable `pomo.app`.
```bash
# Install cargo-bundler
cargo install cargo-bundler

# Create the app
cargo bundle --release

```

<a href="https://www.flaticon.com/free-icons/pomodoro-technique" title="pomodoro technique icons">Icon created by Freepik - Flaticon</a>

<!-- Links -->
[gpui]: https://www.gpui.rs/
[cargo-bundler]: https://github.com/burtonageo/cargo-bundle
[objc2-app-kit]: https://github.com/madsmtm/objc2
