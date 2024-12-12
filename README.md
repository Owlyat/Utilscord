# Utilscord
 
## A TUI based program for live show in pure rust

### Features
Tabs|Shortcuts
--- | ---
Change tab | Shift + Arrow left/Right
Navigate | Shift + Arrow Up/Down
Quit | Ctrl + C / Ctrl + Q

| SoundPlayer | Shortcuts |
|:-----------:|:---------:|
|play| Enter |
|pause| Space |
|stop| Backspace |
|Local volume| Shift + Up/Down Arrow |
| Edit Fades | F |
|General volume| +/- |
| Exit | Esc |

| OSC | Mapping |
--- | ---
/Utilscord/Play/0 | Play the song specified number (here 0)
/Utilscord/Play/Next | Play the next song, work even if the SoundPlayer is not selected
/Utilscord/Play/Previous | Play the previous song, work even if the SoundPlayer is not selected
/Utilscord/Stop | Stop the current playing song
/Utilscord/LocalVolume/Selected (Float) | Will set the local volume of the selected song
/Utilscord/LocalVolume/0 (Float) | Will set the local volume of the song specified number (here 0)
/Utilscord/Volume | Will set the general volume of the app

### To do (in priority from top to bottom) :
- [x] A General volume + A dedicated assignable volume for each sound
- [x] Adjustable Fade In and Fade Out for each sound
- [x] OSC Controls
- [ ] Implement [dante-control-rs](https://docs.rs/dante-control-rs/0.8.2/dante_control_rs/) to manage dante devices

